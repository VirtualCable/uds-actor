import logging
import typing
import asyncio
import sys
import signal

from udsactor import consts, rest, native
from .server import LinuxUDSActorServer
from ..abc import Runner

logger = logging.getLogger(__name__)


def signal_handler(sig, frame):
    logger.debug('Signal handler called with signal %s', sig)
    LinuxUDSActorServer.stop_event.set()  # Signal the server to stop


def usage():
    """Shows usage"""
    sys.stderr.write('usage: udsactor run|login "username"|logout "username"\n')
    sys.exit(2)


async def login(username: str) -> None:
    """
    Logs in a user
    """
    logger.debug('Logging in user %s', username)
    client = rest.PrivateREST()
    r = await client.user_login(
        username=username, sessionType=await native.Manager.instance().operations.getSessionType()
    )
    print('{},{},{},{}\n'.format(r.ip, r.hostname, r.max_idle, r.dead_line or ''))
    # Store session id on /tmp/udsactor.session file, so it can be used by logout if present
    with open(consts.CLIENT_SESSION_ID_FILE, 'w', encoding='utf8') as f:
        f.write(r.session_id or '')


async def logout(username: str) -> None:
    """
    Logs out a user
    """
    logger.debug('Logging out user %s', username)
    client = rest.PrivateREST()
    # Try to get session id from /tmp/udsactor.session file
    with open(consts.CLIENT_SESSION_ID_FILE, 'r', encoding='utf8') as f:
        session_id = f.read()
    await client.user_logout(username=username, session_id=session_id)


class LinuxRunner(Runner):
    def run(self) -> None:
        """
        Main entry point
        Actor is redesigned to run on foreground
        Daemonization is done by systemd, removed from here
        """
        if len(sys.argv) == 3 and sys.argv[1] in ('login', 'logout'):
            # Execute required forced action
            asyncio.run(getattr(sys.modules[__name__], sys.argv[1])(sys.argv[2]))
            sys.exit(0)
        elif len(sys.argv) != 2:
            usage()

        if sys.argv[1] == 'run':
            logger.debug('Starting service')
            # Setup signal handlers for CTRL+C or TERM
            signal.signal(signal.SIGINT, signal_handler)
            signal.signal(signal.SIGTERM, signal_handler)
            # Execute Main thread, and wait for it to finish
            udsAppServer = LinuxUDSActorServer()
            udsAppServer.run()  # Blocking call, not running on a thread
        else:
            usage()