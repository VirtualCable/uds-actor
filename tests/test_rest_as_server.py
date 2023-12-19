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


from udsactor import rest, managed, consts

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

    async def test_logout(self) -> None:
        async with rest_server.setup(token=fake_uds_server.TOKEN) as conn:
            async with conn.client.post(tools.public_rest_path('logout'), ssl=False) as resp:
                data = await self.check_and_get_response(resp, 200, 'logout')
                # Ensure repsonse is an string, contains consts.VERSION and UDS
                self.assertEqual(data, consts.OK)
                
            self.assertEqual(conn.msg_processor.outgoing_queue.qsize(), 1)
            msg = await conn.msg_processor.outgoing_queue.get()
            self.assertEqual(msg.msg_type, managed.types.UDSMessageType.LOGOUT)
            self.assertEqual(msg.data, managed.types.LogoutRequest.null().asDict())

            # Invalid token now, will fail with a forbidden (403)
            async with conn.client.post(tools.public_rest_path('logout', token='invalid'), ssl=False) as resp:
                self.assertEqual(resp.status, 403)
