import logging
import typing
import asyncio
import sys
import signal

from udsactor import consts, rest, native
from .server import LinuxUDSActorServer
from ..abc import Runner

logger = logging.getLogger(__name__)


def signal_handler(sig: typing.Any, frame: typing.Any) -> None:
    logger.debug('Signal handler called with signal %s', sig)
    LinuxUDSActorServer.stop_event.set()  # Signal the server to stop


class LinuxRunner(Runner):
    def run(self) -> None:
        """
        Main entry point
        Actor is redesigned to run on foreground
        Daemonization is done by systemd
        """
        if len(sys.argv) == 1 or len(sys.argv) == 2 and sys.argv[1] in ('run', 'debug'):
            logger.debug('Starting service')
            # Setup signal handlers for CTRL+C or TERM
            signal.signal(signal.SIGINT, signal_handler)
            signal.signal(signal.SIGTERM, signal_handler)
            # Execute Main thread, and wait for it to finish
            udsAppServer = LinuxUDSActorServer()
            udsAppServer.run()  # Blocking call, not running on a thread
        elif len(sys.argv) == 3 and sys.argv[1] in ('login', 'logout'):
            # Execute required forced action
            if sys.argv[1] == 'login':
                self.login(sys.argv[2])
            elif sys.argv[1] == 'logout':
                self.logout(sys.argv[2])
        else:
            self.usage()
            sys.exit(2)

    def usage(self) -> None:
        """Shows usage"""
        sys.stderr.write('usage: udsactor run|login "username"|logout "username"\n')
        sys.exit(2)

    def login(self, username: str) -> None:
        """
        Logs in a user
        """

        async def inner() -> None:
            logger.debug('Logging in user %s', username)
            client = rest.PrivateREST()
            r = await client.user_login(
                username=username, sessionType=await native.Manager.instance().operations.session_type()
            )
            print('{},{},{},{}\n'.format(r.ip, r.hostname, r.max_idle, r.dead_line or ''))
            # Store session id on /tmp/udsactor.session file, so it can be used by logout if present
            with open(consts.CLIENT_SESSION_ID_FILE, 'w', encoding='utf8') as f:
                f.write(r.session_id or '')

        asyncio.run(inner())

    def logout(self, username: str) -> None:
        """
        Logs out a user
        """

        async def inner() -> None:
            logger.debug('Logging out user %s', username)
            client = rest.PrivateREST()
            # Try to get session id from /tmp/udsactor.session file
            with open(consts.CLIENT_SESSION_ID_FILE, 'r', encoding='utf8') as f:
                session_id = f.read()
            await client.user_logout(username=username, session_id=session_id)

        asyncio.run(inner())
