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
    async def test_notify_login_ok(self) -> None:
        priv = rest.PrivateREST(ipv6=False)
        async with fake_uds_server.fake_uds_rest_server() as server:
            async with rest_server.setup(udsserver_port=server.port, token=fake_uds_server.TOKEN) as conn:
                loginResult = await priv.user_login(username='ok', sessionType='sessionType')
                self.assertEqual(loginResult.ip, '0.1.2.3')
                self.assertEqual(loginResult.session_id, 'test_session_id')
                self.assertEqual(loginResult.max_idle, 987654321)
                self.assertEqual(loginResult.dead_line, 123456789)

    async def test_notify_login_hard_fail(self) -> None:
        # Create own event loop queue
        # Mock the "managed.ManagedActorProcessor.login" to raise an exception
        with mock.patch.object(managed.ManagedActorProcessor, 'login', side_effect=Exception('Test exception')):
            priv = rest.PrivateREST(ipv6=False)
            async with fake_uds_server.fake_uds_rest_server() as server:
                async with rest_server.setup(udsserver_port=server.port, token=fake_uds_server.TOKEN) as conn:
                    try:
                        loginResult = await priv.user_login(username='1234', sessionType='sessionType')
                    except Exception as e:
                        self.assertIn('Test exception', str(e))

    async def test_notify_login_fail(self) -> None:
        # Do not start the fake server, so it will fail
        priv = rest.PrivateREST(ipv6=False)
        async with rest_server.setup(udsserver_port=55555, token=fake_uds_server.TOKEN) as conn:
            loginResult = await priv.user_login(username='fails', sessionType='sessionType')
            self.assertTrue(loginResult.is_empty)

    async def test_notify_logout_ok(self) -> None:
        priv = rest.PrivateREST(ipv6=False)
        async with fake_uds_server.fake_uds_rest_server() as server:
            async with rest_server.setup(udsserver_port=server.port, token=fake_uds_server.TOKEN) as conn:
                await priv.user_logout(username='1234', session_id='test_session_id')

    async def test_ws(self) -> None:
        priv = rest.PrivateREST(ipv6=False)
        async with fake_uds_server.fake_uds_rest_server() as server:
            async with rest_server.setup(udsserver_port=server.port, token=fake_uds_server.TOKEN) as conn:
                waiter = asyncio.Event()

                ping_msg = types.UDSMessage(
                    msg_type=types.UDSMessageType.PING,
                )
                ping_response = types.UDSMessage(
                    msg_type=types.UDSMessageType.PONG,
                )

                login_msg = types.UDSMessage(
                    msg_type=types.UDSMessageType.LOGIN,
                    data={'username': '1234', 'session_type': 'sessionType'},
                )

                async def callback(msg: types.UDSMessage) -> None:
                    try:
                        self.assertEqual(msg, ping_response)
                    except Exception as e:
                        logger.exception('Exception on callback')
                    waiter.set()
                    raise exceptions.RequestStop()

                # Execute as a task
                task = asyncio.create_task(priv.communicate(callback))

                # Ensure that the task is running, and no error is raised
                await asyncio.sleep(0.1)
                self.assertFalse(task.done())

                for i in range(10):
                    await priv.send_message(ping_msg)
                    await waiter.wait()
                    waiter.clear()

                # Send a message to the task

    async def test_ws_ping(self) -> None:
        async def processor(msg: types.UDSMessage) -> None:
            self.assertEqual(msg.msg_type, types.UDSMessageType.PONG)
            raise exceptions.RequestStop()

        async with ws.ws_connection(processor, 2) as conn:
            await conn.private.send_message(types.UDSMessage(msg_type=types.UDSMessageType.PING))

            await conn.task

            # Ensure no exceptions
            self.assertEqual(conn.excpts, [])

    async def test_ws_close(self) -> None:
        async def processor(msg: types.UDSMessage) -> None:
            self.assertEqual(msg.msg_type, types.UDSMessageType.OK)
            raise exceptions.RequestStop()

        async with ws.ws_connection(processor, 2) as conn:
            await conn.private.send_message(types.UDSMessage(msg_type=types.UDSMessageType.CLOSE))

            await conn.task

            # Ensure no exceptions
            self.assertEqual(conn.excpts, [])

    async def test_ws_log(self) -> None:
        async def processor(msg: types.UDSMessage) -> None:
            self.assertEqual(msg.msg_type, types.UDSMessageType.OK)
            raise exceptions.RequestStop()

        async with ws.ws_connection(processor, 2) as conn:
            await conn.private.send_message(
                types.UDSMessage(msg_type=types.UDSMessageType.LOG, data={'level': 'INFO', 'message': 'test'})
            )

            await conn.task

            # Ensure no exceptions
            self.assertEqual(conn.excpts, [])
