#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import signal
from unittest import mock

from unittest import IsolatedAsyncioTestCase

from udsactor import native

from .utils import tools

import udsactor_service


class TestManagedServer(IsolatedAsyncioTestCase):
    async def asyncSetUp(self) -> None:
        tools.skip_if_not('linux')

    async def test_managed_server(self) -> None:
        # Step 0 is read the config, communicate with UDS Broker and execute the required action
        # Setup the manager to patch the runner
        manager = native.Manager.instance()
        manager.runner = mock.Mock()

        # Executing main must start the runner (runner.run is called)
        with mock.patch('sys.argv', ['udsactor_service']):
            udsactor_service.main()
            manager.runner.run.assert_called_with()

    async def test_linux_service_arguments(self) -> None:
        # Specific imports for linux, test will be skipped if not linux (on setUp)
        from udsactor.native.linux import server, service

        manager = native.Manager.instance()
        # Patch usage and test that is is invoked if args are not correct
        # Also, patch sys.exit to avoid exit from test
        with mock.patch('sys.argv', ['udsactor_service', 'xxxxx']):
            with mock.patch('sys.exit') as exit_mock:
                with mock.patch.object(service.LinuxRunner, 'usage') as usage_mock:
                    manager.runner.run()
                    usage_mock.assert_called_with()
                # ensure exit code is 2
                exit_mock.assert_called_with(2)

        # Now, ensure run and debug works (that is, run is called and signal is setup)
        # Patch signal setup, and LinuxUDSActorServer.run
        for arg in ('run', 'debug', ''):
            arglist = ['udsactor_service'] if arg == '' else ['udsactor_service', arg]

            with mock.patch('sys.argv', arglist):
                with mock.patch('signal.signal') as signal_mock:
                    with mock.patch.object(server.LinuxUDSActorServer, 'run') as run_mock:
                        manager.runner.run()
                        # Ensure signal is setup for TERM and INT
                        signal_mock.assert_any_call(signal.SIGTERM, mock.ANY)
                        signal_mock.assert_any_call(signal.SIGINT, mock.ANY)
                        # Ensure run is called
                        run_mock.assert_called_with()

        # Ensure login and logout works
        # Login and logout are called with username as argument
        for arg in ('login', 'logout'):
            arglist = ['udsactor_service', arg, 'username']

            with mock.patch('sys.argv', arglist):
                with mock.patch.object(service.LinuxRunner, 'login') as login_mock:
                    with mock.patch.object(service.LinuxRunner, 'logout') as logout_mock:
                        manager.runner.run()
                        if arg == 'login':
                            login_mock.assert_called_with('username')
                            logout_mock.assert_not_called()
                        else:
                            logout_mock.assert_called_with('username')
                            login_mock.assert_not_called()

    