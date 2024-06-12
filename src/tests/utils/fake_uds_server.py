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
# pyright: reportUnusedFunction=false
import aiohttp
import aiohttp.web
import typing
import random
import string
import time
import logging
import contextlib


from udsactor import consts, cert
from .cert import generate_cert

from .tools import get_free_port

logger = logging.getLogger('fake_uds_server')


# Fake UDS REST Server, providing the server part of our rest api for testing purposes
TOKEN: typing.Final[str] = ''.join(random.SystemRandom().choices(string.ascii_letters + string.digits, k=40))

AUTHENTICATORS: typing.Final[list[dict[str, typing.Any]]] = [
    {
        "authId": "81d05b90-7790-5ef8-a2de-c63dda19a09b",
        "authSmallName": "ad2019",
        "auth": "Sample With AD",
        "type": "ActiveDirectoryAuthenticator",
        "priority": 1,
        "isCustom": False,
    },
    {
        "authId": "9EB0689D-DF66-54FF-8E7A-3C11E3F42A1A",
        "authSmallName": "differ",
        "auth": "OtherInternal",
        "type": "InternalDBAuth",
        "priority": 1,
        "isCustom": False,
    },
    {
        "authId": "9803fc06-d8b3-5f11-9a6e-eec905c017fd",
        "authSmallName": "172.27.0.1:8000",
        "auth": "Internal",
        "type": "InternalDBAuth",
        "priority": 2,
        "isCustom": False,
    },
]


def rest_result(result: typing.Any, **kwargs: typing.Any) -> aiohttp.web.Response:
    '''
    Returns a REST result
    '''
    # A common possible value in kwargs is "error"
    return aiohttp.web.json_response(
        {'result': result, 'stamp': time.time(), 'version': consts.VERSION, **kwargs}
    )


@contextlib.asynccontextmanager
async def fake_uds_rest_server() -> typing.AsyncIterator['FakeUDSRestServer']:
    '''
    Context manager that starts a fake UDS REST server and returns it
    '''
    server = FakeUDSRestServer()
    await server.start()
    try:
        yield server
    finally:
        await server.stop()


class FakeUDSRestServer:
    requests: list[tuple[str, typing.Any]]
    site: typing.Optional[aiohttp.web.TCPSite]
    webapp: typing.Optional[aiohttp.web.Application]
    port: int = 0  # Will contain Fake server port

    def __init__(self) -> None:
        self.requests = []
        self.site = None
        self.webapp = None

    async def store_request(self, request: aiohttp.web.Request) -> None:
        try:
            json = await request.json()
        except Exception:
            json = None
        self.requests.append(
            (
                request.path,
                {
                    'headers': dict(request.headers),
                    'json': json,
                    'query': dict(request.query),
                },
            )
        )

    def clear_requests(self) -> None:
        self.requests.clear()

    host = property(lambda self: self.site.name.split('//')[1].rstrip('/'))

    async def start(self) -> None:
        """
        Runs the server and returns the url where it is running
        """
        routes = aiohttp.web.RouteTableDef()

        @routes.get('/uds/rest/auth/auths')
        async def auth_authenticators(request: aiohttp.web.Request) -> aiohttp.web.Response:
            await self.store_request(request)
            return aiohttp.web.json_response(AUTHENTICATORS)

        @routes.post('/uds/rest/auth/login')
        async def auth_login(request: aiohttp.web.Request) -> aiohttp.web.Response:
            await self.store_request(request)
            return rest_result('ok', token=TOKEN)

        @routes.post('/uds/rest/actor/v3/test')
        async def test(request: aiohttp.web.Request) -> aiohttp.web.Response:
            params = await request.json()
            # Expects token and type (managed or unmanaged)
            if params['token'] != TOKEN:
                return rest_result('invalid token', error='Invalid token')

            if 'type' not in params:
                return rest_result('error', error='Invalid type')

            # Store request data for later inspection
            await self.store_request(request)
            return rest_result('ok')  # Same token as login, in fact, does not matter... :)

        @routes.post('/uds/rest/actor/v3/register')
        async def register(request: aiohttp.web.Request) -> aiohttp.web.Response:
            _params = await request.json()
            await self.store_request(request)
            return rest_result(TOKEN)  # Same token as login, in fact, does not matter... :)

        @routes.post('/uds/rest/actor/v3/initialize')
        async def initialize(request: aiohttp.web.Request) -> aiohttp.web.Response:
            params = await request.json()
            # Expects token, type, version, build and id
            if params['token'] != TOKEN:
                return rest_result('invalid token', error='Invalid token')

            for i in ('type', 'version', 'build', 'id'):
                if i not in params:
                    return rest_result('error', error=f'Invalid: {i} is missing')

            # Store request data for later inspection
            await self.store_request(request)

            return rest_result(
                {
                    'own_token': TOKEN,  # For testing, keep same token
                    'token': TOKEN,  # For testing, keep same token
                    'unique_id': params['id'][0]['mac'],
                    'os': {'action': 'rename', 'name': 'test_name'},
                }
            )

        @routes.post('/uds/rest/actor/v3/ready')
        async def ready(request: aiohttp.web.Request) -> aiohttp.web.Response:
            params = await request.json()
            # Expects token, secret, ip and port
            if params['token'] != TOKEN:
                return rest_result('invalid token', error='Invalid token')

            for i in ('secret', 'ip', 'port'):
                if i not in params:
                    return rest_result('error', error=f'Invalid: {i} is missing')

            # cert = generate_cert(params['ip'])

            # Store request data for later inspection
            await self.store_request(request)

            # Return stable values for testing
            return rest_result(
                {
                    'private_key': 'test_key',
                    'key': 'test_key',
                    'server_certificate': 'test_certificate',
                    'certificate': 'test_certificate',
                    'password': 'test_password',
                    'ciphers': 'test_ciphers',
                }
            )

        @routes.post('/uds/rest/actor/v3/unmanaged')
        async def unmanaged(request: aiohttp.web.Request) -> aiohttp.web.Response:
            params = await request.json()

            # Expects token, secret, id and port
            if params['token'] != TOKEN:
                return rest_result('invalid token', error='Invalid token')

            for i in ('secret', 'id', 'port'):
                if i not in params:
                    return rest_result('error', error=f'Invalid: {i} is missing')

            # cert = generate_cert(params['ip'])

            # Store request data for later inspection
            await self.store_request(request)

            # Return stable values for testing
            return rest_result(
                {
                    'private_key': 'test_key',
                    'key': 'test_key',
                    'server_certificate': 'test_certificate',
                    'certificate': 'test_certificate',
                    'password': 'test_password',
                    'ciphers': 'test_ciphers',
                }
            )

        @routes.post('/uds/rest/actor/v3/ipchange')
        async def ipchange(request: aiohttp.web.Request) -> aiohttp.web.Response:
            params = await request.json()

            # Expects token, secret, ip and port
            if params['token'] != TOKEN:
                return rest_result('invalid token', error='Invalid token')

            for i in ('secret', 'ip', 'port'):
                if i not in params:
                    return rest_result('error', error=f'Invalid: {i} is missing')

            # cert = generate_cert(params['ip'])

            # Store request data for later inspection
            await self.store_request(request)

            # Return stable values for testing
            return rest_result(
                {
                    'private_key': 'test_key',
                    'key': 'test_key',
                    'server_certificate': 'test_certificate',
                    'certificate': 'test_certificate',
                    'password': 'test_password',
                    'ciphers': 'test_ciphers',
                }
            )

        @routes.post('/uds/rest/actor/v3/login')
        async def login(request: aiohttp.web.Request) -> aiohttp.web.Response:
            params = await request.json()
            # Expects token, type, username and session_type
            if params['token'] != TOKEN:
                return rest_result('invalid token', error='Invalid token')

            for i in ('type', 'username', 'session_type'):
                if i not in params:
                    return rest_result('error', error=f'Invalid: {i} is missing')

            # Store request data for later inspection
            await self.store_request(request)

            return rest_result(
                {
                    'ip': '0.1.2.3',
                    'hostname': 'test_hostname',
                    'dead_line': 123456789,
                    'max_idle': 987654321,
                    'session_id': 'test_session_id',
                }
            )

        @routes.post('/uds/rest/actor/v3/logout')
        async def logout(request: aiohttp.web.Request) -> aiohttp.web.Response:
            params = await request.json()

            # expects token, type, username, session_type and session_id
            if params['token'] != TOKEN:
                return rest_result('invalid token', error='Invalid token')

            for i in ('type', 'username', 'session_type', 'session_id'):
                if i not in params:
                    return rest_result('error', error=f'Invalid: {i} is missing')

            # Store request data for later inspection
            await self.store_request(request)

            return rest_result('ok')

        @routes.post('/uds/rest/actor/v3/log')
        async def log(request: aiohttp.web.Request) -> aiohttp.web.Response:
            params = await request.json()

            # expects token, level and message
            if params['token'] != TOKEN:
                return rest_result('invalid token', error='Invalid token')

            for i in ('level', 'message'):
                if i not in params:
                    return rest_result('error', error=f'Invalid: {i} is missing')

            # Store request data for later inspection
            await self.store_request(request)

            return rest_result('ok')

        ssl_context = cert.generate_server_ssl_context(generate_cert('127.0.0.1'))

        self.webapp = aiohttp.web.Application(logger=logger)

        self.webapp.add_routes(routes)
        runner = aiohttp.web.AppRunner(self.webapp)
        await runner.setup()

        self.port = get_free_port()
        self.site = aiohttp.web.TCPSite(
            runner=runner,
            host='127.0.0.1',
            port=self.port,
            ssl_context=ssl_context,
            reuse_address=True,
        )
        await self.site.start()

    async def stop(self) -> None:
        if self.site:
            await self.site.stop()
            self.site = None
        if self.webapp:
            await self.webapp.shutdown()
            await self.webapp.cleanup()
            self.webapp = None
