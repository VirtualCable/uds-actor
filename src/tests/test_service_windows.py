#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
from unittest import mock

from unittest import IsolatedAsyncioTestCase

from udsactor import native

from .utils import tools

import udsactor_service


class TestManagedServerWindows(IsolatedAsyncioTestCase):
    async def asyncSetUp(self) -> None:
        tools.skip_if_not('windows')

    async def test_managed_server(self) -> None:
        # Step 0 is read the config, communicate with UDS Broker and execute the required action
        # Setup the manager to patch the runner
        manager = native.Manager.instance()
        manager.runner = mock.Mock()

        # Executing main must start the runner (runner.run is called)
        with mock.patch('sys.argv', ['udsactor_service']):
            udsactor_service.main()
            manager.runner.run.assert_called_with()

    async def test_windows_service_startup(self) -> None:
        from udsactor.native.windows import service, server

        manager = native.Manager.instance()

        with mock.patch('sys.argv', ['udsactor_service', 'debug']):
            with mock.patch.object(server.WindowsUDSActorServer, 'run') as run_mock:
                manager.runner.run()
                run_mock.assert_called_with()

        with mock.patch('sys.argv', ['udsactor_service', '--setup-recovery']):
            with mock.patch.object(service.WindowsUDSActorServer, 'setup_recovery') as setup_mock:
                manager.runner.run()
                setup_mock.assert_called_with()

        # Mock the "servicemanager" module methods:
        #  * Initialize
        #  * PrepareToHostSingle
        #  * StartServiceCtrlDispatcher
        with mock.patch('servicemanager.Initialize') as init_mock:
            with mock.patch('servicemanager.PrepareToHostSingle') as prepare_mock:
                with mock.patch('servicemanager.StartServiceCtrlDispatcher') as dispatcher_mock:
                    with mock.patch('sys.argv', ['udsactor_service']):
                        manager.runner.run()
                        init_mock.assert_called_once_with()
                        prepare_mock.assert_called_once_with(service.UDSActorService)
                        dispatcher_mock.assert_called_once_with()

        # Mock HandleCommandLine from "win32serviceutil" module and test install, remove, start and stop
        with mock.patch('win32serviceutil.HandleCommandLine') as handle_mock:
            for arg in ('install', 'remove', 'start', 'stop'):
                with mock.patch('sys.argv', ['udsactor_service', arg]):
                    manager.runner.run()
                    handle_mock.assert_called_with(service.UDSActorService)
                    handle_mock.reset_mock()
