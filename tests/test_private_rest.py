#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import asyncio
import logging
from unittest import mock


from udsactor import exceptions, rest, managed, consts, types

from .utils import rest_server, fixtures, fake_uds_server, exclusive_tests, ws

logger = logging.getLogger(__name__)

# Also, due to the fact that there are more than one event loop, we need to ensure that
# the test is run alone and comms.Queue is not shared between event loops
# Also, the configuration is keep in a singleton global variable, so we need to ensure
# that those kind of tests are run alone


class TestPrivateRest(exclusive_tests.AsyncExclusiveTests):
    async def test_ws_ping(self) -> None:
        async def processor(msg: types.UDSMessage) -> None:
            self.assertEqual(msg.msg_type, types.UDSMessageType.PONG)
            raise exceptions.RequestStop()

        async with ws.ws_connection(processor) as conn:
            await conn.private.send_message(types.UDSMessage(msg_type=types.UDSMessageType.PING))

            await conn.task

            # Ensure no exceptions
            self.assertEqual(conn.excpts, [])

    async def test_ws_close(self) -> None:
        async def processor(msg: types.UDSMessage) -> None:
            # Should not be called
            raise Exception('Should not be called')

        async with ws.ws_connection(processor) as conn:
            # Override close processor
            called: asyncio.Event = asyncio.Event()

            async def _replacement(*args, **kwargs) -> None:
                try:
                    self.assertEqual(kwargs['username'], '')
                    self.assertEqual(kwargs['session_type'], '')
                    self.assertEqual(kwargs['session_id'], '')
                except Exception as e:
                    conn.excpts.append(e)
                called.set()

            conn.local_server.msg_processor.actor.logout = _replacement

            await conn.private.send_message(types.UDSMessage(msg_type=types.UDSMessageType.CLOSE))

            await asyncio.wait_for(called.wait(), 8)

            # Ensure no exceptions
            self.assertEqual(conn.excpts, [])

    async def test_ws_log(self) -> None:
        async def processor(msg: types.UDSMessage) -> None:
            # Should not be called
            raise Exception('Should not be called')

        async with ws.ws_connection(processor) as conn:
            # Override close processor
            called: asyncio.Event = asyncio.Event()

            async def _replacement(*args, **kwargs) -> None:
                try:
                    self.assertEqual(kwargs['level'], types.LogLevel.INFO)
                    self.assertEqual(kwargs['message'], 'test')
                except Exception as e:
                    conn.excpts.append(e)
                called.set()

            conn.local_server.msg_processor.actor.log = _replacement

            await conn.private.send_message(
                types.UDSMessage(
                    msg_type=types.UDSMessageType.LOG,
                    data=types.LogRequest(level=types.LogLevel.INFO, message='test').asDict(),
                )
            )

            await asyncio.wait_for(called.wait(), 8)

            # Ensure no exceptions
            self.assertEqual(conn.excpts, [])

    async def test_ws_login(self) -> None:
        loginResult: types.LoginResponse = types.LoginResponse.null()

        async def processor(msg: types.UDSMessage) -> None:
            nonlocal loginResult
            self.assertEqual(msg.msg_type, types.UDSMessageType.LOGIN)
            loginResult = types.LoginResponse(**msg.data)
            raise exceptions.RequestStop()

        async with ws.ws_connection(processor) as conn:
            # Override close processor
            called: asyncio.Event = asyncio.Event()

            async def _replacement(*args, **kwargs) -> types.LoginResponse:
                called.set()
                # TODO: test args
                try:
                    self.assertEqual(kwargs['username'], '1234')
                    self.assertEqual(kwargs['session_type'], 'test')
                except Exception as e:
                    conn.excpts.append(e)
                return types.LoginResponse(
                    ip='0.1.2.3',
                    hostname='test',
                    dead_line=123456789,
                    max_idle=987654321,
                    session_id='test_session_id',
                )

            conn.local_server.msg_processor.actor.login = _replacement

            await conn.private.send_message(
                types.UDSMessage(
                    msg_type=types.UDSMessageType.LOGIN,
                    data=types.LoginRequest(username='1234', session_type='test').asDict(),
                )
            )

            await asyncio.wait_for(called.wait(), 8)

            # Now we should get a loginResult

            await conn.task

            # Ensure no exceptions
            self.assertEqual(conn.excpts, [])

            # We should receive the login message response on loginResult
            self.assertEqual(loginResult.ip, '0.1.2.3')
            self.assertEqual(loginResult.session_id, 'test_session_id')
            self.assertEqual(loginResult.max_idle, 987654321)
            self.assertEqual(loginResult.dead_line, 123456789)

    async def test_ws_logout(self) -> None:
        async def processor(msg: types.UDSMessage) -> None:
            # Should not be called
            raise Exception('Should not be called')

        async with ws.ws_connection(processor, 3) as conn:
            # Override close processor
            called: asyncio.Event = asyncio.Event()

            async def _replacement(*args, **kwargs) -> None:
                try:
                    self.assertEqual(kwargs['username'], '1234')
                    self.assertEqual(kwargs['session_type'], '')
                    self.assertEqual(kwargs['session_id'], 'session_id')
                except Exception as e:
                    conn.excpts.append(e)
                called.set()

            conn.local_server.msg_processor.logged_in = True
            conn.local_server.msg_processor.actor.logout = _replacement

            await conn.private.send_message(
                types.UDSMessage(
                    msg_type=types.UDSMessageType.LOGOUT,
                    data=types.LogoutRequest(username='1234', session_id='session_id').asDict(),
                )
            )

            await asyncio.wait_for(called.wait(), 8)

            # Ensure no exceptions
            self.assertEqual(conn.excpts, [])

            # Now, second call will not re set the event
            called.clear()
            await conn.private.send_message(
                types.UDSMessage(
                    msg_type=types.UDSMessageType.LOGOUT,
                    data=types.LogoutRequest(username='1234', session_id='session_id').asDict(),
                )
            )

            await conn.task

            # Ensure called is not set
            self.assertFalse(called.is_set())

            # Enusure 1 Exception is produced (timeout)
            self.assertEqual(len(conn.excpts), 1)
            self.assertIsInstance(conn.excpts[0], asyncio.TimeoutError)
