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
# pylint: disable=invalid-name

import socket
import time
import secrets
import subprocess
import typing
import collections.abc
import logging
import threading
import asyncio

from udsactor import types, comms, managed, unmanaged, rest, platform, server_msg_processor as msg_processor, log

logger = logging.getLogger(__name__)

if typing.TYPE_CHECKING:
    from udsactor.abc import ActorProcessor


# Actor server runs on its own thread, so we can use asyncio.run() to run the main task
# And keep the service running until the main finishes (or the service is stopped)
class UDSActorServer(threading.Thread):
    def __init__(self) -> None:
        super().__init__()

    async def _run(self):
        # Run the mainAsyncTask, and store the task to check if it has finished
        task = asyncio.create_task(self.main())

        while not comms.stopEvent.is_set():
            await asyncio.sleep(1)
            # Check if the task has finished
            if task.done():
                comms.stopEvent.set()

                # Try to get result of main task
                try:
                    task.result()
                except asyncio.CancelledError:
                    pass
                except Exception as e:
                    logger.exception(e)

        tasks = [tsk for tsk in asyncio.all_tasks() if tsk != asyncio.current_task()]
        # Cancel all not finished tasks on the event loop
        for tsk in tasks:
            if not tsk.done():
                tsk.cancel()

        # Wait for all tasks to finish with a timeout of 5 seconds for all tasks
        await asyncio.wait(tasks, timeout=5)

        # Get tasks results, looking for exceptions
        for tsk in tasks:
            try:
                tsk.result()
            except asyncio.CancelledError:  # Since 3.8, CancelledError is not an Exception, but a BaseException
                pass
            except Exception as e:
                logger.exception(e)

    def run(self):
        logger.debug('Starting UDSActorServer')

        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)

        try:
            loop.run_until_complete(self._run())
        except Exception as e:
            logger.exception(e)

        logger.debug('Stopping UDSActorServer')

        # Ensure service knows that we are done. (an unhanded exception could have stoppped our loop without setting the event)
        if not comms.stopEvent.is_set():
            comms.stopEvent.set()  # Ensure stopEvent is set

    async def main(self) -> None:
        cfg = await platform.Platform.platform().config

        # Not configured, simply stop
        if not cfg.is_empty():
            logger.info('UDS Actor is not configured. stopping service')
            return

        # Setup actor processor
        actor: 'ActorProcessor'
        if cfg.actorType == types.ActorType.MANAGED:
            actor = managed.ManagedActorProcessor()
        else:
            actor = unmanaged.UnmanagedActorProcessor()

        # Keep reference to tasks so we can cancel them on exit (and avoid garbage collection of them)
        backTasks: typing.Final[list[asyncio.Task]] = [
            # asyncio.create_task(platform.events.sensEventsProcessor(cfg)),  # Add events processor task
            # asyncio.create_task(platform.events.statsNotifier(cfg)),  # Add stats notifier task
            asyncio.create_task(log.UDSBrokerLogger.waitAndSendLogs()),  # Add log sender task
            asyncio.create_task(
                msg_processor.MessagesProcessor(actor=actor).run()
            ),  # Add message processor task
        ]
        # First, wait for interfaces to be available BEFORE trying to initialize anything
        logger.info('Waiting for network connectivity')
        while True:
            interfaces = await platform.Platform.platform().operations.validNetworkCards()
            if len(interfaces) > 0:
                break

        logger.info('Detected network interfaces: %s', interfaces)

        certInfo: typing.Optional[types.CertificateInfo] = await actor.initialize(interfaces)

        # Create the webserver and run until cancelled
        try:
            # await webserver.server(cfg)  # Run web server, if it fails, stop full service
            pass
        except asyncio.CancelledError:
            for task in backTasks:
                task.cancel()

            # Wait for all tasks to finish after cancellation
            await asyncio.gather(*backTasks, return_exceptions=True)
