# -*- coding: utf-8 -*-
#
# Copyright (c) 2024 Virtual Cable S.L.U.
# All rights reserved.
#
# Redistribution and use in source and binary forms, with or without modification,
# are permitted provided that the following conditions are met:
#
#    * Redistributions of source code must retain the above copyright notice,
#      this list of conditions and the following disclaimer.
#    * Redistributions in binary form must reproduce the above copyright notice,
#      this list of conditions and the following disclaimer in the documentation
#      and/or other materials provided with the distribution.
#    * Neither the name of Virtual Cable S.L.U. nor the names of its contributors
#      may be used to endorse or promote products derived from this software
#      without specific prior written permission.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
# AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
# IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
# DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
# FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
# DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
# SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
# CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
# OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
# OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

'''
@author: Adolfo Gómez, dkmaster at dkmon dot com
'''
# pyright: reportUnknownMemberType=false,reportMissingModuleSource=false
import logging
import typing
import sys

import win32serviceutil
import win32service
import pythoncom
import servicemanager

# So pyinstaller can find these modules, not used here
import win32timezone  # pyright: ignore [reportUnusedImport]
import win32security  # pyright: ignore [reportUnusedImport]
import win32net  # pyright: ignore [reportUnusedImport]
import win32event  # pyright: ignore [reportUnusedImport]

from .server import WindowsUDSActorServer
from ..abc import Runner

logger = logging.getLogger(__name__)

SVC_NAME: typing.Final[str] = 'UDSActorNG'


class UDSActorService(win32serviceutil.ServiceFramework):
    # ServiceeFramework related
    _svc_name_ = SVC_NAME
    _svc_display_name_ = 'UDS Actor Service'
    _svc_description_ = 'UDS Actor Management Service'
    _svc_deps_ = ['EventLog']

    def __init__(self, args: typing.Any) -> None:
        win32serviceutil.ServiceFramework.__init__(self, args)
        # Create an event which we will use to wait on.
        # The "service stop" request will set this event.
        # self.stopEvent = threading.Event()

    def SvcStop(self):
        # Before we do anything, tell the SCM we are starting the stop process.
        self.ReportServiceStatus(win32service.SERVICE_STOP_PENDING)

        WindowsUDSActorServer.stop_event.set()

    SvcShutdown = SvcStop

    def SvcDoRun(self):
        try:
            # Notifies the SCM that the service is started
            servicemanager.LogMsg(
                servicemanager.EVENTLOG_INFORMATION_TYPE,
                servicemanager.PYS_SERVICE_STARTED,
                (self._svc_name_, ''),
            )

            logger.debug('Starting service')
            logger.debug('Initializing coms')
            pythoncom.CoInitialize()

            # Launch the UDSAppServer
            udsActorServer = WindowsUDSActorServer()
            udsActorServer.start()

            # Register SENS events
            # While hWaitStop hasn't been set by SvcStop, we loop and do stuff.
            while True:
                # Wait for service stop signal, if timeout, loop again
                if WindowsUDSActorServer.stop_event.wait(0.1) == True:
                    break

                # Processes pending messages
                # This will be a bit asyncronous (0.1 second delay, from previous wait)
                pythoncom.PumpWaitingMessages()

            logger.debug('Stopping service')

            # Wait for the UDSAppServer to stop
            udsActorServer.join()

            # Unregister SENS events (not really needed, but...)
            # SENS.SensLogon.unregister()

            # Notifies the SCM that the service is stopped
            servicemanager.LogMsg(
                servicemanager.EVENTLOG_INFORMATION_TYPE,
                servicemanager.PYS_SERVICE_STOPPED,
                (self._svc_name_, ''),
            )
        except Exception as e:
            logger.error('Exception in SvcDoRun')
            logger.exception(e)


class WindowsRunner(Runner):
    def run(self) -> None:
        """
        Runs the service (Allows service installation, removal, etc..)
        """
        logger.debug('Starting service - Outside')
        # If started as a service, no extra arguments are passed.
        # We make this this way so that we can run the service from an .exe generated by pyinstaller.
        if len(sys.argv) == 1:
            logger.debug('Starting service')
            servicemanager.Initialize()
            servicemanager.PrepareToHostSingle(UDSActorService)
            servicemanager.StartServiceCtrlDispatcher()
        elif sys.argv[1] == '--setup-recovery':  # len(sys.argv) is greater than 1
            self.setup_recovery()
        elif sys.argv[1] == 'debug':
            # Execute as application, not as service (for debugging purposes on windows)
            udsAppServer = WindowsUDSActorServer()
            udsAppServer.run()  # Blocking call, not running on a thread
        else:
            win32serviceutil.HandleCommandLine(UDSActorService)

    # def stop(self) -> None:
    #     """
    #     tries to stop the service
    #     """
    #     win32serviceutil.StopService(SVC_NAME)

    def setup_recovery(self):
        svc_name = UDSActorService._svc_name_  # pyright: ignore [reportPrivateUsage]

        hs: typing.Any = None
        hscm = None
        try:
            hscm = win32service.OpenSCManager(None, None, win32service.SC_MANAGER_ALL_ACCESS)

            try:
                hs = win32serviceutil.SmartOpenService(hscm, svc_name, win32service.SERVICE_ALL_ACCESS)
                service_failure_actions = {
                    'ResetPeriod': 864000,  # Time in ms after which to reset the failure count to zero.
                    'RebootMsg': u'',  # Not using reboot option
                    'Command': u'',  # Not using run-command option
                    'Actions': [
                        (win32service.SC_ACTION_RESTART, 5000),  # action, delay in ms
                        (win32service.SC_ACTION_RESTART, 5000),
                    ],
                }
                win32service.ChangeServiceConfig2(
                    hs, win32service.SERVICE_CONFIG_FAILURE_ACTIONS, service_failure_actions
                )
            finally:
                if hs:
                    win32service.CloseServiceHandle(hs)
        finally:
            if hscm:
                win32service.CloseServiceHandle(hscm)
