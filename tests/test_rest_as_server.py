#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import typing
import asyncio
import logging
import aiohttp
from unittest import mock


from udsactor import rest, managed, consts, types

from .utils import rest_server, fixtures, fake_uds_server, exclusive_tests, tools

# Also, due to the fact that there are more than one event loop, we need to ensure that
# the test is run alone and comms.Queue is not shared between event loops
# Also, the configuration is keep in a singleton global variable, so we need to ensure
# that those kind of tests are run alone

logger = logging.getLogger(__name__)


class TestPublicRest(exclusive_tests.AsyncExclusiveTests):
    async def check_and_get_response(self, resp: aiohttp.ClientResponse, code: int, message: str) -> typing.Any:
        self.assertEqual(resp.status, code, msg=f'{message} - {resp.reason}')
        data = await resp.json()
        msg = f'{message} - {data}'
        self.assertIsInstance(data, dict, msg=msg)
        self.assertIn('result', data, msg=msg)
        self.assertIn('stamp', data, msg=msg)
        self.assertIn('version', data, msg=msg)
        self.assertEqual(data['version'], consts.VERSION, msg=msg)
        logger.debug('Response %s: %s', message, data)
        return data['result']

    async def test_information(self) -> None:
        async with rest_server.setup(token=fake_uds_server.TOKEN) as conn:
            async with conn.client.get(tools.public_rest_path('information'), ssl=False) as resp:
                data = await self.check_and_get_response(resp, 200, 'information')
                # Ensure repsonse is an string, contains consts.VERSION and UDS
                self.assertIn('UDS', data)

            # Invalid token now, will fail with a forbidden (403)
            async with conn.client.get(
                tools.public_rest_path('information', token='invalid'), ssl=False
            ) as resp:
                self.assertEqual(resp.status, 403)

    # Post method
    async def test_logout(self) -> None:
        async with rest_server.setup(token=fake_uds_server.TOKEN) as conn:
            async with conn.client.post(tools.public_rest_path('logout'), ssl=False) as resp:
                data = await self.check_and_get_response(resp, 200, 'logout')
                # Ensure repsonse is an string, contains consts.VERSION and UDS
                self.assertEqual(data, consts.OK)

            self.assertEqual(conn.msg_processor.outgoing_queue.qsize(), 1)
            msg = await conn.msg_processor.outgoing_queue.get()
            self.assertEqual(msg.msg_type, managed.types.UDSMessageType.LOGOUT)
            self.assertEqual(msg.data, managed.types.LogoutRequest.null().as_dict())

            # Invalid token now, will fail with a forbidden (403)
            async with conn.client.post(tools.public_rest_path('logout', token='invalid'), ssl=False) as resp:
                self.assertEqual(resp.status, 403)

    # Post method
    async def test_message(self) -> None:
        async with rest_server.setup(token=fake_uds_server.TOKEN) as conn:
            async with conn.client.post(
                tools.public_rest_path('message'), json={'message': 'test'}, ssl=False
            ) as resp:
                data = await self.check_and_get_response(resp, 200, 'message')
                # Ensure repsonse is an string, contains consts.VERSION and UDS
                self.assertEqual(data, consts.OK)

            self.assertEqual(conn.msg_processor.outgoing_queue.qsize(), 1)
            msg = await conn.msg_processor.outgoing_queue.get()
            self.assertEqual(msg.msg_type, managed.types.UDSMessageType.MESSAGE)
            self.assertEqual(msg.data, 'test')

            # Invalid token now, will fail with a forbidden (403)
            async with conn.client.post(
                tools.public_rest_path('message', token='invalid'), json={'message': 'test'}, ssl=False
            ) as resp:
                self.assertEqual(resp.status, 403)

    # Post method
    async def test_preconnect(self) -> None:
        # Tests preconnect using types.PreconnectRequest on a post
        test_data = types.PreconnectRequest(
            username='test_user',
            protocol='test_protocol',
            hostname='test_host',
            ip='0.1.2.3',
            udsuser='test_udsuser',
        )
        # Check compat method an current method
        for route in ('preconnect', 'preConnect'):
            async with rest_server.setup(token=fake_uds_server.TOKEN) as conn:
                for compat in (False, True):
                    async with conn.client.post(
                        tools.public_rest_path(route),
                        json=test_data.as_dict(compat=compat),
                        ssl=False,
                    ) as resp:
                        data = await self.check_and_get_response(resp, 200, route)
                        # Ensure repsonse is an string, contains consts.VERSION and UDS
                        self.assertEqual(data, consts.OK)

                    self.assertEqual(conn.msg_processor.outgoing_queue.qsize(), 1)
                    msg = await conn.msg_processor.outgoing_queue.get()
                    self.assertEqual(msg.msg_type, managed.types.UDSMessageType.PRECONNECT)
                    self.assertEqual(types.PreconnectRequest.from_dict(msg.data), test_data)

                # Invalid token now, will fail with a forbidden (403)
                async with conn.client.post(
                    tools.public_rest_path('message', token='invalid'), json={'message': 'test'}, ssl=False
                ) as resp:
                    self.assertEqual(resp.status, 403)

    # Post method
    async def test_screenshot(self) -> None:
        # Test screenshot, no data
        async with rest_server.setup(token=fake_uds_server.TOKEN) as conn:
            async with conn.client.post(tools.public_rest_path('screenshot'), ssl=False) as resp:
                data = await self.check_and_get_response(resp, 200, 'screenshot')
                # Ensure repsonse is an string, contains consts.VERSION and UDS
                self.assertEqual(data, consts.OK)

            self.assertEqual(conn.msg_processor.outgoing_queue.qsize(), 1)
            msg = await conn.msg_processor.outgoing_queue.get()
            self.assertEqual(msg.msg_type, managed.types.UDSMessageType.SCREENSHOT)
            self.assertEqual(msg.data, {})

            # Invalid token now, will fail with a forbidden (403)
            async with conn.client.post(
                tools.public_rest_path('screenshot', token='invalid'), ssl=False
            ) as resp:
                self.assertEqual(resp.status, 403)

    # Post method
    async def test_script(self) -> None:
        test_script = types.ScriptRequest(script='# Python script', script_type='python')
        async with rest_server.setup(token=fake_uds_server.TOKEN) as conn:
            async with conn.client.post(
                tools.public_rest_path('script'), json=test_script.as_dict(), ssl=False
            ) as resp:
                data = await self.check_and_get_response(resp, 200, 'script')
                # Ensure repsonse is an string, contains consts.VERSION and UDS
                self.assertEqual(data, consts.OK)

            self.assertEqual(conn.msg_processor.outgoing_queue.qsize(), 1)
            msg = await conn.msg_processor.outgoing_queue.get()
            self.assertEqual(msg.msg_type, managed.types.UDSMessageType.SCRIPT)
            self.assertEqual(msg.data, test_script.as_dict())

            # Invalid token now, will fail with a forbidden (403)
            async with conn.client.post(
                tools.public_rest_path('script', token='invalid'), json=test_script.as_dict(), ssl=False
            ) as resp:
                self.assertEqual(resp.status, 403)

    # Get method, cheks returned uuid is cfg.token
    async def test_uuid(self) -> None:
        async with rest_server.setup(token=fake_uds_server.TOKEN) as conn:
            async with conn.client.get(tools.public_rest_path('uuid'), ssl=False) as resp:
                data = await self.check_and_get_response(resp, 200, 'uuid')
                # Ensure repsonse is an string, contains consts.VERSION and UDS
                self.assertEqual(data, fake_uds_server.TOKEN)

            # Invalid token now, will fail with a forbidden (403)
            async with conn.client.get(
                tools.public_rest_path('uuid', token='invalid'), ssl=False
            ) as resp:
                self.assertEqual(resp.status, 403)