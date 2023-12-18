# -*- coding: utf-8 -*-
#
# Copyright (c) 2023 Virtual Cable S.L.U.
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
#    * Neither the name of Virtual Cable S.L. nor the names of its contributors
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
Author: Adolfo Gómez, dkmaster at dkmon dot com
'''
import logging
import random
import typing
import collections.abc

from unittest import IsolatedAsyncioTestCase

from udsactor import consts, rest, types

from .utils import fake_uds_server

logger = logging.getLogger(__name__)

TEST_INTERFACES = [
    types.InterfaceInfo(name='eth0', mac='00:00:00:00:00:00', ip='0.0.0.1'),
    types.InterfaceInfo(name='eth1', mac='00:00:00:00:00:01', ip='0.0.0.2'),
    types.InterfaceInfo(name='eth2', mac='00:00:00:00:00:02', ip='0.0.0.3'),
]

# Currently, BrokerREST api does not uses any shared configuration, so we can run all tests in parallel
# If this changes, we need to ensure that only one test is run at a time by using the "exclusive_tests.AsyncExclusiveTests" class
# as base class for the test class

class TestRestAsClient(IsolatedAsyncioTestCase):
    async def test_enum_auths(self) -> None:
        async with fake_uds_server.fake_uds_rest_server() as server:
            rest_api = rest.BrokerREST(server.host, False)
            auths = await rest_api.enumerateAuthenticators()
            self.assertEqual(len(auths), len(fake_uds_server.AUTHENTICATORS))
            auths = sorted(auths, key=lambda x: x.authId)
            authenticators = sorted(fake_uds_server.AUTHENTICATORS, key=lambda x: x['authId'])
            for i in range(len(auths)):
                self.assertEqual(auths[i].auth, authenticators[i]['auth'])
                self.assertEqual(auths[i].authId, authenticators[i]['authId'])
                self.assertEqual(auths[i].authSmallName, authenticators[i]['authSmallName'])
                self.assertEqual(auths[i].isCustom, authenticators[i]['isCustom'])
                self.assertEqual(auths[i].priority, authenticators[i]['priority'])
                self.assertEqual(auths[i].type, authenticators[i]['type'])

    async def test_register(self) -> None:
        async with fake_uds_server.fake_uds_rest_server() as server:
            rest_api = rest.BrokerREST(server.host, False)
            authId = random.choice(fake_uds_server.AUTHENTICATORS)['authId']

            token = await rest_api.register(
                auth=authId,
                username='test',
                password='test',
                hostname='test_hostname',
                ip='127.1.1.1',
                mac='00:00:00:00:00:00',
                preCommand='preCommand',
                runOnceCommand='runOnceCommand',
                postCommand='postCommand',
                logLevel=1,
            )
            self.assertEqual(token, fake_uds_server.TOKEN)
            self.assertEqual(len(server.requests), 2)  # One for login, one for register
            self.assertEqual(server.requests[0][0], '/uds/rest/auth/login')
            self.assertEqual(server.requests[1][0], '/uds/rest/actor/v3/register')

    async def test_test(self) -> None:
        async with fake_uds_server.fake_uds_rest_server() as server:
            rest_api = rest.BrokerREST(server.host, False, token=fake_uds_server.TOKEN)
            for atype in types.ActorType:
                server.clear_requests()
                ok = await rest_api.test(type=atype)
                self.assertTrue(ok)
                self.assertEqual(len(server.requests), 1)
                self.assertEqual(server.requests[0][0], '/uds/rest/actor/v3/test')
                self.assertEqual(
                    server.requests[0][1]['json'], {'token': fake_uds_server.TOKEN, 'type': atype.value}
                )

    async def test_event_log(self) -> None:
        async with fake_uds_server.fake_uds_rest_server() as server:
            rest_api = rest.BrokerREST(server.host, False, token=fake_uds_server.TOKEN)
            # Log returns nothing
            await rest_api.log(types.LogLevel.INFO, 'test message')
            self.assertEqual(len(server.requests), 1)
            self.assertEqual(server.requests[0][0], '/uds/rest/actor/v3/log')
            self.assertEqual(
                server.requests[0][1]['json'],
                {'token': fake_uds_server.TOKEN, 'level': types.LogLevel.INFO.value, 'message': 'test message'},
            )

    async def test_initialize(self) -> None:
        for atype in types.ActorType:
            async with fake_uds_server.fake_uds_rest_server() as server:
                server.clear_requests()
                rest_api = rest.BrokerREST(server.host, False, token=fake_uds_server.TOKEN)
                # Log returns nothing
                resp = await rest_api.initialize(interfaces=TEST_INTERFACES, actor_type=atype)
                self.assertEqual(resp.token, fake_uds_server.TOKEN)
                self.assertEqual(resp.unique_id, TEST_INTERFACES[0].mac)
                self.assertEqual(
                    resp.os, types.ActorOsConfiguration(action='rename', name='test_name', custom=None)
                )
                self.assertEqual(len(server.requests), 1)

                self.assertEqual(server.requests[0][0], '/uds/rest/actor/v3/initialize')
                js = server.requests[0][1]['json']
                self.assertEqual(js['token'], fake_uds_server.TOKEN)
                self.assertEqual(js['type'], atype.value)

    async def test_initialize_unmanaged(self) -> None:
        async with fake_uds_server.fake_uds_rest_server() as server:
            server.clear_requests()
            rest_api = rest.BrokerREST(server.host, False, token=fake_uds_server.TOKEN)
            # Log returns nothing
            resp = await rest_api.initialize_unmanaged(
                interfaces=TEST_INTERFACES, port=1212
            )
            self.assertIsInstance(resp, types.CertificateInfo)
            self.assertIsNotNone(resp)
            self.assertEqual(resp.certificate, 'test_certificate')
            self.assertEqual(resp.ciphers, 'test_ciphers')
            self.assertEqual(resp.key, 'test_key')
            self.assertEqual(resp.password, 'test_password')

            self.assertEqual(len(server.requests), 1)
            self.assertEqual(server.requests[0][0], '/uds/rest/actor/v3/unmanaged')
            js = server.requests[0][1]['json']
            self.assertEqual(js['token'], fake_uds_server.TOKEN)
            self.assertEqual(js['secret'], consts.OWN_AUTH_TOKEN)
            self.assertEqual(js['port'], 1212)

    async def test_ready(self) -> None:
        async with fake_uds_server.fake_uds_rest_server() as server:
            rest_api = rest.BrokerREST(server.host, False, token=fake_uds_server.TOKEN)
            # Login event returns types.LoginResult
            resp = await rest_api.ready(ip='0.1.2.3', port=1212)
            self.assertIsInstance(resp, types.CertificateInfo)
            self.assertIsNotNone(resp)
            self.assertEqual(resp.certificate, 'test_certificate')
            self.assertEqual(resp.ciphers, 'test_ciphers')
            self.assertEqual(resp.key, 'test_key')
            self.assertEqual(resp.password, 'test_password')

            self.assertEqual(len(server.requests), 1)
            self.assertEqual(server.requests[0][0], '/uds/rest/actor/v3/ready')
            self.assertEqual(
                server.requests[0][1]['json'],
                {'token': fake_uds_server.TOKEN, 'secret': consts.OWN_AUTH_TOKEN, 'ip': '0.1.2.3', 'port': 1212},
            )

    async def test_notify_new_ip(self) -> None:
        async with fake_uds_server.fake_uds_rest_server() as server:
            rest_api = rest.BrokerREST(server.host, False, token=fake_uds_server.TOKEN)
            # Login event returns types.LoginResult
            resp = await rest_api.notify_new_ip(ip='0.1.2.3', port=1212)
            self.assertIsInstance(resp, types.CertificateInfo)
            self.assertIsNotNone(resp)
            self.assertEqual(resp.certificate, 'test_certificate')
            self.assertEqual(resp.ciphers, 'test_ciphers')
            self.assertEqual(resp.key, 'test_key')
            self.assertEqual(resp.password, 'test_password')

            self.assertEqual(len(server.requests), 1)
            self.assertEqual(server.requests[0][0], '/uds/rest/actor/v3/ipchange')
            self.assertEqual(
                server.requests[0][1]['json'],
                {'token': fake_uds_server.TOKEN, 'secret': consts.OWN_AUTH_TOKEN, 'ip': '0.1.2.3', 'port': 1212},
            )

    async def test_notify_login(self) -> None:
        for atype in types.ActorType:
            async with fake_uds_server.fake_uds_rest_server() as server:
                rest_api = rest.BrokerREST(server.host, False, token=fake_uds_server.TOKEN)
                # Login event returns types.LoginResult
                resp = await rest_api.notify_login(
                    actor_type=atype,
                    username='test',
                    session_type='test_session_type',
                )
                self.assertIsInstance(resp, types.LoginResponse)
                self.assertIsNotNone(resp)
                self.assertEqual(resp.ip, '0.1.2.3')
                self.assertEqual(resp.hostname, 'test_hostname')
                self.assertEqual(resp.dead_line, 123456789)
                self.assertEqual(resp.max_idle, 987654321)
                self.assertEqual(resp.session_id, 'test_session_id')

                self.assertEqual(len(server.requests), 1)
                self.assertEqual(server.requests[0][0], '/uds/rest/actor/v3/login')
                self.assertEqual(
                    server.requests[0][1]['json'],
                    {
                        'token': fake_uds_server.TOKEN,
                        'type': atype.value,
                        'username': 'test',
                        'session_type': 'test_session_type',
                    },
                )

    async def test_notify_logout(self) -> None:
        for atype in types.ActorType:
            async with fake_uds_server.fake_uds_rest_server() as server:
                rest_api = rest.BrokerREST(server.host, False, token=fake_uds_server.TOKEN)
                # Login event returns types.LoginResult
                resp = await rest_api.notify_logout(
                    actor_type=atype,
                    username='test',
                    session_id='test_session_id',
                    session_type='test_session_type',
                )
                self.assertEqual(resp, 'ok')

                self.assertEqual(len(server.requests), 1)
                self.assertEqual(server.requests[0][0], '/uds/rest/actor/v3/logout')
                self.assertEqual(
                    server.requests[0][1]['json'],
                    {
                        'token': fake_uds_server.TOKEN,
                        'type': atype.value,
                        'username': 'test',
                        'session_id': 'test_session_id',
                        'session_type': 'test_session_type',
                    },
                )
