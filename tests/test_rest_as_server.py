#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import asyncio
from unittest import mock


from udsactor import rest, managed, consts

from .utils import rest_server, fixtures, fake_uds_server, exclusive_tests

# Also, due to the fact that there are more than one event loop, we need to ensure that
# the test is run alone and comms.Queue is not shared between event loops
# Also, the configuration is keep in a singleton global variable, so we need to ensure
# that those kind of tests are run alone

class TestPublicRest(exclusive_tests.AsyncExclusiveTests):
    async def test_information(self) -> None:
        return
        async with rest_server.setup(token=fake_uds_server.TOKEN) as conn:
            async with conn.get('/information') as resp:
                self.assertEqual(resp.status, 200)
                data = await resp.json()
                # Ensure repsonse is an string, contains consts.VERSION and UDS
