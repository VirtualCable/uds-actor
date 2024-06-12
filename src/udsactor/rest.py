#!/usr/bin/env python3
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
#    * Neither the name of Virtual Cable S.L.U. nor the names of its contributors
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
import asyncio
import json
import logging
import ssl
import typing
import collections.abc

import aiohttp
import aiohttp.client_exceptions

from . import consts, exceptions, types, security

logger = logging.getLogger(__name__)


#
# Basic UDS Api, needed for our purposes
#
class BrokerREST:  # pylint: disable=too-few-public-methods
    """
    Base for remote api accesses
    """

    _host: str
    _verify_ssl: bool
    _url: str
    _context: 'ssl.SSLContext'
    _token: str | None = None

    def __init__(self, host: str, verify_ssl: bool, token: str | None = None) -> None:
        self._host = host
        self._verify_ssl = verify_ssl
        self._url = f'https://{self._host}/uds/rest/'
        self._token = token
        # try:
        #     warnings.simplefilter("ignore")  # Disables all warnings
        # except Exception:
        #     pass

        self._context = security.create_client_sslcontext(verify=verify_ssl)

    @property
    def _headers(self) -> typing.MutableMapping[str, str]:
        return {
            'Content-Type': 'application/json',
            'User-Agent': 'UDS AppServer v{}'.format(consts.VERSION),
        }

    @property
    def token(self) -> str:
        if not self._token:
            raise exceptions.RESTError('Token not provided')

        return self._token

    @token.setter
    def token(self, value: str | None) -> None:
        self._token = value

    def _url_for(self, api: types.ApiType, method: str) -> str:
        if api == types.ApiType.AUTH:
            return self._url + 'auth/' + method
        # Actor v3
        return self._url + 'actor/v3/' + method

    async def _do_post(
        self,
        api: types.ApiType,
        method: str,  # i.e. 'initialize', 'ready', ....
        payLoad: collections.abc.Mapping[str, typing.Any],
        headers: typing.Optional[collections.abc.Mapping[str, str]] = None,
        checkError: bool = True,
        returnRaw: bool = False,
    ) -> typing.Any:
        headers = headers or self._headers
        result = None
        data = ''
        async with aiohttp.ClientSession() as session:
            try:
                result = await session.post(
                    self._url_for(api, method),
                    data=json.dumps(payLoad),
                    headers=headers,
                    ssl=self._context,
                    timeout=consts.TIMEOUT,
                )
                if result.ok:
                    j = await result.json()
                    if checkError and j.get('error', None):
                        raise exceptions.RESTError(j['error'])
                    if returnRaw:
                        return j
                    return j['result']
            except aiohttp.client_exceptions.ClientConnectorError as e:
                raise exceptions.RESTConnectionError(str(e))
            except TimeoutError as e:
                data = f'Timeout processing request to {self._host}'
            except Exception as e:
                data = str(e)

            # Error, get it back
            if result and not data:
                try:
                    data = await result.json()
                except Exception:
                    data = await result.text()

            raise exceptions.RESTError(data)

    async def _do_get(
        self,
        api: types.ApiType,
        method: str,  # i.e. 'initialize', 'ready', ....
        headers: typing.Optional[collections.abc.Mapping[str, str]] = None,
        check_error: bool = True,
        return_raw: bool = False,
    ) -> typing.Any:
        headers = headers or self._headers
        result = None
        data = ''
        async with aiohttp.ClientSession() as session:
            try:
                result = await session.get(
                    self._url_for(api, method),
                    headers=headers,
                    ssl=self._context,
                    timeout=consts.TIMEOUT,
                )
                if result.ok:
                    j = await result.json()
                    if check_error and j.get('error', None):
                        raise exceptions.RESTError(j['error'])
                    if return_raw:
                        return j
                    return j['result']
            except aiohttp.client_exceptions.ClientConnectorError as e:
                raise exceptions.RESTConnectionError(str(e))
            except Exception as e:
                data = str(e)

            # Error, get it back
            if result and not data:
                try:
                    data = await result.json()
                except Exception:
                    data = await result.text()

            raise exceptions.RESTError(data)

    async def enumerate_authenticators(self) -> list[types.Authenticator]:
        try:
            return sorted(
                [
                    types.Authenticator(
                        authId=v['authId'],
                        authSmallName=v['authSmallName'],
                        auth=v['auth'],
                        type=v['type'],
                        priority=v['priority'],
                        isCustom=v['isCustom'],
                    )
                    for v in sorted(
                        await self._do_get(types.ApiType.AUTH, 'auths', check_error=False, return_raw=True),
                        key=lambda x: x['auth'],
                    )
                ],
                key=lambda x: x.auth.lower(),
            )
        except Exception:
            raise  # To be "pass" in future, if cannot enumerate, it is not a problem

    async def login(self, auth: str, username: str, password: str) -> typing.MutableMapping[str, str]:
        """
        Raises an exception if could not login, or returns the "authorization token"

        Args:
            auth: Authenticator to use
            username: Username to use
            password: Password to use on login
        """
        try:
            result = await self._do_post(
                types.ApiType.AUTH,
                'login',
                {'auth': auth, 'username': username, 'password': password},
                returnRaw=True,
            )

            return {'X-Auth-Token': result['token']}
        except Exception:
            raise exceptions.RESTError('Invalid credentials')

    async def register(
        self,
        auth: str,
        username: str,
        password: str,
        hostname: str,
        ip: str,
        mac: str,
        preCommand: str,
        runOnceCommand: str,
        postCommand: str,
        logLevel: int,
    ) -> str:
        """
        Raises an exception if could not register, or registers and returns the "authorization token"

        Args:
            auth: Authenticator to use
            username: Username to use
            password: Password to use on login
            hostname: Hostname of this machine
            ip: IP of this machine
            mac: Mac of this machine
            preCommand: Command to execute before a new connection
            runOnceCommand: Command to execute only once
            postCommand: Command to execute after machine is configured
            logLevel: Log level to use
        """
        data = {
            'username': username + '@' + auth,
            'hostname': hostname,
            'ip': ip,
            'mac': mac,
            'pre_command': preCommand,
            'run_once_command': runOnceCommand,
            'post_command': postCommand,
            'log_level': (logLevel * 10000) + 20000,
        }

        # First, try to login to REST api
        headers = await self.login(auth, username, password)
        return await self._do_post(
            types.ApiType.ACTORV3,
            'register',
            payLoad=data,
            headers=headers,
        )

    async def test(self, type: types.ActorType) -> bool:
        """Test if token is valid

        Args:
            token: Token to test

        Returns:
            True if token is valid, False otherwise
        """
        return (
            await self._do_post(types.ApiType.ACTORV3, 'test', payLoad={'token': self.token, 'type': type})
        ) == 'ok'

    # The flow for initialization of an actor is:
    # * Actor is started
    # * Actor gets interfaces
    # * Actor calls initialize with the interfaces
    # * Actor gets a token and a unique_id
    # * Actor calls ready with the token, ip and port
    # * Actor gets a certificate
    # * Actor starts webserver
    # * User logins
    # ....
    async def initialize(
        self,
        interfaces: collections.abc.Iterable[types.InterfaceInfo],
        actor_type: typing.Optional[str],
    ) -> types.InitializationResult:
        # Generate id list from netork cards
        payload = {
            'type': actor_type or types.ActorType.MANAGED,
            'token': self.token,
            'version': consts.VERSION,
            'build': consts.BUILD,
            'id': [{'mac': i.mac, 'ip': i.ip} for i in interfaces],
        }
        r: dict[str, typing.Any] = await self._do_post(types.ApiType.ACTORV3, 'initialize', payload)
        os = r['os']
        # * TO BE REMOVED ON FUTURE VERSIONS *
        # To keep compatibility, store old values on custom data
        # This will be removed in future versions
        # The values stored are:
        #        username=os.get('username'),
        #        password=os.get('password'),
        #        new_password=os.get('new_password'),
        #        domain=os.get('ad'),
        #        ou=os.get('ou'),
        # So update custom data with this info
        custom = os.get('custom', {})
        for i in ('username', 'password', 'new_password', 'ad', 'ou'):
            # ad is converted to domain
            if i not in os:
                continue  # Skip if not present on os, do not overwrite custom
            name = 'domain' if i == 'ad' else i
            custom[name] = os[i]  # os[i] is present, so force it on custom

        return types.InitializationResult(
            token=r['token'],
            unique_id=r['unique_id'].lower() if r['unique_id'] else None,
            os=(
                types.ActorOsConfiguration(
                    action=os['action'],
                    name=os['name'],
                    custom=os.get('custom'),
                )
                if r['os']
                else None
            ),
        )

    # The flow for initialization of an unmanaged actor is:
    # * Actor is started
    # * Actor gets interfaces
    # * Actor calls initialize_unmanaged with the interfaces, secret and port
    # * Actor gets the certificate
    # * Actor starts webserver
    # * User logins
    # * NOW the initialize is called, (Now we have an userService assigned, not before)
    # ....
    async def initialize_unmanaged(
        self,
        interfaces: collections.abc.Iterable[types.InterfaceInfo],
        port: int,
    ) -> types.CertificateInfo:
        payload = {
            'id': [{'mac': i.mac, 'ip': i.ip} for i in interfaces],
            'token': self.token,
            'secret': consts.OWN_AUTH_TOKEN,
            'port': port,
        }
        result = await self._do_post(types.ApiType.ACTORV3, 'unmanaged', payload)

        return types.CertificateInfo.from_dict(result)

    async def ready(self, ip: str, port: int) -> types.CertificateInfo:
        payload = {'token': self.token, 'secret': consts.OWN_AUTH_TOKEN, 'ip': ip, 'port': port}
        result = await self._do_post(types.ApiType.ACTORV3, 'ready', payload)

        return types.CertificateInfo.from_dict(result)

    async def notify_new_ip(self, ip: str, port: int) -> types.CertificateInfo:
        payload = {'token': self.token, 'secret': consts.OWN_AUTH_TOKEN, 'ip': ip, 'port': port}
        result = await self._do_post(types.ApiType.ACTORV3, 'ipchange', payload)

        return types.CertificateInfo.from_dict(result)

    async def notify_login(
        self,
        actor_type: str,
        username: str,
        session_type: str,
    ) -> types.LoginResponse:
        payload = {
            'type': actor_type,
            'token': self.token,
            'username': username,
            'session_type': session_type,
        }
        result = await self._do_post(types.ApiType.ACTORV3, 'login', payload)
        return types.LoginResponse.from_dict(result)

    async def notify_logout(
        self,
        actor_type: str,
        username: str,
        session_id: str,
        session_type: str,
    ) -> typing.Optional[str]:
        payload = {
            'type': actor_type,
            'token': self.token,
            'username': username,
            'session_type': session_type,
            'session_id': session_id,
        }
        return await self._do_post(types.ApiType.ACTORV3, 'logout', payload)  # Can be 'ok' or 'notified'

    async def log(self, level: types.LogLevel, message: str) -> None:
        """Sends a log message to UDS Server

        Args:
            token: Token to use
            level: Level of log message
            message: Message to send
            user_service: If present, the uuid of the user service that is sending the message
        """
        payLoad = {
            'token': self.token,
            'level': level.value,
            'message': message,
        }
        await self._do_post(
            types.ApiType.ACTORV3,
            'log',
            payLoad=payLoad,
        )

    # Helper to execute async rest in sync mode
    # For use with configurator
    @staticmethod
    def syncexec(
        f: collections.abc.Callable[..., typing.Any], *args: typing.Any, **kwargs: typing.Any
    ) -> typing.Any:
        try:
            loop = asyncio.get_event_loop()  # Get or create event loop
            return loop.run_until_complete(f(*args, **kwargs))
        except Exception as e:
            logger.error('Exception on sync: %s', e)
            raise


#
# Client Connection to APP Server using REST and asyncio
#
class PrivateREST:  # pylint: disable=too-few-public-methods
    """
    Base for remote api accesses
    """

    _queue: asyncio.Queue[types.UDSMessage]

    _session_id: str = ''
    _session_type: str = ''

    _url: str
    _context: 'ssl.SSLContext'

    def __init__(self, ipv6: bool = False) -> None:
        self._queue = asyncio.Queue()

        self._url = f'https://{"127.0.0.1" if not ipv6 else "[::1]"}:{consts.LISTEN_PORT}{consts.BASE_PRIVATE_REST_PATH}/'

        self._context = ssl._create_unverified_context(  # pyright: ignore [reportPrivateUsage]
            purpose=ssl.Purpose.SERVER_AUTH, check_hostname=False
        )

        # Disable SSLv2, SSLv3, TLSv1, TLSv1.1, TLSv1.2
        self._context.minimum_version = ssl.TLSVersion.TLSv1_3  # Local connection, no need for more

    @property
    def _headers(self) -> typing.MutableMapping[str, str]:
        return {
            'Content-Type': 'application/json',
            'User-Agent': 'UDS Actor Client v{}'.format(consts.VERSION),
        }

    def _urlFor(self, method: str) -> str:
        return self._url + method

    async def _do_post(
        self,
        method: str,  # i.e. 'initialize', 'ready', ....
        payLoad: collections.abc.Mapping[str, typing.Any],
        headers: typing.Optional[collections.abc.Mapping[str, str]] = None,
    ) -> typing.Any:
        headers = headers or self._headers
        result = None
        data = ''
        async with aiohttp.ClientSession() as session:
            try:
                result = await session.post(
                    self._urlFor(method),
                    data=json.dumps(payLoad),
                    headers=headers,
                    ssl=self._context,
                    timeout=consts.TIMEOUT,
                )
                if result.ok:
                    j = await result.json()
                    return j
            except aiohttp.client_exceptions.ClientConnectorError as e:
                raise exceptions.RESTConnectionError(str(e))
            except Exception as e:
                data = str(e)

            # Error, get it back
            if result and not data:
                try:
                    data = await result.json()
                except Exception:
                    data = await result.text()

            raise exceptions.RESTError(data)

    async def communicate(
        self, processor: collections.abc.Callable[[types.UDSMessage], collections.abc.Awaitable[None]]
    ) -> None:
        """Communicates with server using websocket
        This method will call processor for each message received from server

        Args:
            processor: An async callable that will receive the message as a string, and returns nothing

        Note:
            This method will not return until cancelled, or "exceptione.RequesStop" is raised
            from the processor callable.
        """

        async def process_incoming(ws: aiohttp.ClientWebSocketResponse) -> None:
            async for msg in ws:
                if msg.type == aiohttp.WSMsgType.TEXT:
                    await processor(types.UDSMessage(**msg.json()))
                elif msg.type in (aiohttp.WSMsgType.ERROR, aiohttp.WSMsgType.CLOSE):
                    await processor(types.UDSMessage(msg_type=types.UDSMessageType.CLOSE))

        async def proces_outgoing(ws: aiohttp.ClientWebSocketResponse) -> None:
            while True:
                msg = await self._queue.get()
                if typing.cast(typing.Any, msg) is None:  # None is a "stop" message
                    break
                await ws.send_json(msg.as_dict())

        try:
            async with aiohttp.ClientSession() as session:
                async with session.ws_connect(self._urlFor('ws'), ssl=self._context) as ws:
                    await asyncio.gather(process_incoming(ws), proces_outgoing(ws))
            # Note that if server closes connections, but we have not "cancelled" this task, it will retry
            # forever, so we need to sleep a bit before retrying also
        except exceptions.RequestStop:
            # Stop requested, fine...
            logger.info('RequestStop received, stopping')
        except asyncio.CancelledError:
            # Cancel received, propagate it (fine also :)
            raise
        except Exception as e:
            logger.error('Exception on communicate: %s', e)
            raise  # Not expected, so raise it

    async def send_message(self, msg: types.UDSMessage) -> None:
        await self._queue.put(msg)

    async def user_login(self, username: str, sessionType: typing.Optional[str] = None) -> types.LoginResponse:
        self._session_type = sessionType or consts.UNKNOWN
        payload = {
            'username': username,
            'session_type': self._session_type,
        }
        result = await self._do_post('user_login', payload)

        try:
            res = types.LoginResponse.from_dict(result)
            self._session_id = res.session_id or ''
            return res
        except Exception:
            raise Exception('Invalid ticket received from UDS Broker.')

    async def user_logout(self, username: str, session_id: str) -> None:
        payload = {
            'username': username,
            'session_type': self._session_type or consts.UNKNOWN,
            'session_id': self._session_id,  # We now know the session id, provided on login
        }
        await self._do_post('user_logout', payload)  # Can be 'ok' or 'notified'

    async def log(self, level: types.LogLevel, message: str, userservice_uuid: str | None = None) -> None:
        payLoad = {'userservice_uuid': userservice_uuid, 'level': level.value, 'message': message}
        await self._do_post('log', payLoad)  # Ignores result...
