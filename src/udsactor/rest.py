# -*- coding: utf-8 -*-
#
# Copyright (c) 2019-2023 Virtual Cable S.L.U.
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
@author: Adolfo Gómez, dkmaster at dkmon dot com
@author: Alexander Burmatov,  thatman at altlinux dot org
'''
# pylint: disable=invalid-name
import json
import logging
import typing
import platform

import requests

from udsactor import types, tools
from udsactor.version import VERSION, BUILD
from udsactor.decorators import retry_on_exception


# Default public listen port
LISTEN_PORT: typing.Final[int] = 43910

# Default timeout
TIMEOUT: typing.Final[int] = 5  # 5 seconds is more than enought

# Constants
UNKNOWN: typing.Final[str] = 'unknown'


class RESTError(Exception):
    ERRCODE = 0


class RESTConnectionError(RESTError):
    ERRCODE = -1


# Errors ""raised"" from broker
class RESTInvalidKeyError(RESTError):
    ERRCODE = 1


class RESTUnmanagedHostError(RESTError):
    ERRCODE = 2


class RESTUserServiceNotFoundError(RESTError):
    ERRCODE = 3


class RESTOsManagerError(RESTError):
    ERRCODE = 4


# For avoid proxy on localhost connections
NO_PROXY = {
    'http': None,
    'https': None,
}

UDS_BASE_URL = 'https://{}/uds/rest/'


#
# Basic UDS Api
#
class UDSApi:  # pylint: disable=too-few-public-methods
    """
    Base for remote api accesses
    """

    _host: str = ''
    _check_certificate: bool = True
    _url: str = ''
    _session: 'requests.Session'
    _timeout: int = TIMEOUT

    def __init__(self, host: str, check_certificate: bool, timeout: int = TIMEOUT) -> None:
        self._host = host
        self._check_certificate = check_certificate
        self._url = UDS_BASE_URL.format(self._host)
        self._timeout = timeout

        # Disable logging requests messages except for errors, ...
        logging.getLogger('request').setLevel(logging.CRITICAL)
        logging.getLogger('urllib3').setLevel(logging.ERROR)

        self._session = tools.secure_requests_session(verify=self._check_certificate)

    @property
    def _headers(self) -> typing.MutableMapping[str, str]:
        return {
            'Content-Type': 'application/json',
            'User-Agent': 'UDS Actor v{}/{}'.format(VERSION, BUILD),
        }

    def _api_url(self, method: str) -> str:
        raise NotImplementedError

    def _doPost(
        self,
        method: str,  # i.e. 'initialize', 'ready', ....
        payload: typing.MutableMapping[str, typing.Any],
        headers: typing.Optional[typing.MutableMapping[str, str]] = None,
        no_proxy: bool = False,
    ) -> typing.Any:
        headers = headers or self._headers
        try:
            result = self._session.post(
                self._api_url(method),
                data=json.dumps(payload),
                headers=headers,
                timeout=self._timeout,
                proxies=NO_PROXY if no_proxy else None,  # type: ignore  # if not proxies wanted, enforce it
            )

            if result.ok:
                j = result.json()
                if not j.get('error', None):
                    return j['result']
        except requests.ConnectionError as e:
            raise RESTConnectionError(str(e))
        except Exception as e:
            raise RESTError(str(e))

        try:
            data = result.json()
        except Exception:
            data = result.content.decode()

        raise RESTError(data)


#
# UDS Broker API access
#
class UDSServerApi(UDSApi):
    def _api_url(self, method: str) -> str:
        return self._url + 'actor/v3/' + method

    def enumerateAuthenticators(self) -> typing.Iterable[types.AuthenticatorType]:
        try:
            result = self._session.get(
                self._url + 'auth/auths',
                headers=self._headers,
                # verify=self._validateCert,
                timeout=4,
            )
            if result.ok:
                for v in sorted(result.json(), key=lambda x: x['priority']):
                    yield types.AuthenticatorType(
                        authId=v['authId'],
                        authSmallName=v['authSmallName'],
                        auth=v['auth'],
                        type=v['type'],
                        priority=v['priority'],
                        isCustom=v['isCustom'],
                    )
        except Exception:
            pass

    def register(
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
        """
        data: typing.Dict[str, 'str|int'] = {
            'username': username + '@' + auth,
            'hostname': hostname,
            'ip': ip,
            'mac': mac,
            'pre_command': preCommand,
            'version': f'{VERSION}/{BUILD}',
            'run_once_command': runOnceCommand,
            'post_command': postCommand,
            'log_level': logLevel,
            'os': f'{platform.system()}',
        }

        # First, try to login to REST api
        try:
            # First, try to login
            authInfo = {'auth': auth, 'username': username, 'password': password}
            headers = self._headers
            result = self._session.post(
                self._url + 'auth/login',
                data=json.dumps(authInfo),
                headers=headers,
                # verify=self._validateCert,
            )
            if not result.ok or result.json()['result'] == 'error':
                raise Exception()  # Invalid credentials

            headers['X-Auth-Token'] = result.json()['token']

            result = self._session.post(
                self._api_url('register'),
                data=json.dumps(data),
                headers=headers,
                # verify=self._validateCert,
            )
            if result.ok:
                return result.json()['result']
        except requests.ConnectionError as e:
            raise RESTConnectionError(e)
        except RESTError:
            raise
        except Exception as e:
            raise RESTError('Invalid credentials')

        raise RESTError(result.content.decode())

    def initialize(
        self,
        token: str,
        interfaces: typing.Iterable[types.InterfaceInfoType],
        actor_type: typing.Optional[str],
    ) -> types.InitializationResultType:
        # Generate id list from netork cards
        payload: typing.Dict[str, typing.Any] = {
            'type': actor_type or types.MANAGED,
            'token': token,
            'version': VERSION,
            'build': BUILD,
            'id': [{'mac': i.mac, 'ip': i.ip} for i in interfaces],
        }
        r = self._doPost('initialize', payload)
        os = r['os']
        if r['token'] is None:
            raise RESTUnmanagedHostError('Unmanaged host')

        return types.InitializationResultType(
            master_token=r['master_token'],
            token=r['token'],
            unique_id=r['unique_id'].lower() if r['unique_id'] else None,
            os=(
                types.ActorOsConfigurationType(
                    action=os['action'],
                    name=os['name'],
                    custom=os.get('custom'),
                )
                if r['os']
                else None
            ),
        )

    @retry_on_exception(3, wait_seconds=2, retryable_exceptions=[RESTConnectionError], do_log=True)
    def ready(self, own_token: str, secret: str, ip: str, port: int) -> types.CertificateInfoType:
        payload: typing.Dict[str, typing.Any] = {'token': own_token, 'secret': secret, 'ip': ip, 'port': port}
        result = self._doPost('ready', payload)

        return types.CertificateInfoType(
            private_key=result['private_key'],
            server_certificate=result['server_certificate'],
            password=result['password'],
            ciphers=result.get('ciphers', ''),
        )

    @retry_on_exception(3, wait_seconds=2, retryable_exceptions=[RESTConnectionError], do_log=True)
    def notifyIpChange(self, own_token: str, secret: str, ip: str, port: int) -> types.CertificateInfoType:
        payload: typing.Dict[str, typing.Any] = {'token': own_token, 'secret': secret, 'ip': ip, 'port': port}
        result = self._doPost('ipchange', payload)

        return types.CertificateInfoType(
            private_key=result['private_key'],
            server_certificate=result['server_certificate'],
            password=result['password'],
            ciphers=result.get('ciphers', ''),
        )

    @retry_on_exception(3, wait_seconds=2, retryable_exceptions=[RESTConnectionError], do_log=True)
    def notifyUnmanagedCallback(
        self,
        master_token: str,
        secret: str,
        interfaces: typing.Iterable[types.InterfaceInfoType],
        port: int,
    ) -> types.CertificateInfoType:
        payload: typing.Dict[str, typing.Any] = {
            'id': [{'mac': i.mac, 'ip': i.ip} for i in interfaces],
            'token': master_token,
            'secret': secret,
            'port': port,
        }
        result = self._doPost('unmanaged', payload)

        return types.CertificateInfoType(
            private_key=result['private_key'],
            server_certificate=result['server_certificate'],
            password=result['password'],
            ciphers=result.get('ciphers', ''),
        )

    @retry_on_exception(3, wait_seconds=2, retryable_exceptions=[RESTConnectionError], do_log=True)
    def login(
        self,
        actor_type: typing.Optional[str],
        token: str,
        username: str,
        session_type: str,
        interfaces: typing.Iterable[types.InterfaceInfoType],
        secret: typing.Optional[str],
    ) -> types.LoginResultInfoType:
        if not token:
            return types.LoginResultInfoType(
                ip='0.0.0.0', hostname=UNKNOWN, deadline=None, max_idle=None, session_id=None
            )
        payload: typing.Dict[str, typing.Any] = {
            'type': actor_type or types.MANAGED,
            'id': [{'mac': i.mac, 'ip': i.ip} for i in interfaces],
            'token': token,
            'username': username,
            'session_type': session_type,
            'secret': secret or '',
        }
        result = self._doPost('login', payload)
        return types.LoginResultInfoType(
            ip=result['ip'],
            hostname=result['hostname'],
            deadline=result['deadline'],
            max_idle=result['max_idle'],
            session_id=result.get('session_id', ''),
        )

    @retry_on_exception(3, wait_seconds=2, retryable_exceptions=[RESTConnectionError], do_log=True)
    def logout(
        self,
        actor_type: typing.Optional[str],
        token: str,
        username: str,
        session_id: str,
        session_type: str,
        interfaces: typing.Iterable[types.InterfaceInfoType],
        secret: typing.Optional[str],
    ) -> typing.Optional[str]:
        if not token:
            return None
        payload: typing.Dict[str, typing.Any] = {
            'type': actor_type or types.MANAGED,
            'id': [{'mac': i.mac, 'ip': i.ip} for i in interfaces],
            'token': token,
            'username': username,
            'session_type': session_type,
            'session_id': session_id,
            'secret': secret or '',
        }
        return self._doPost('logout', payload)  # Can be 'ok' or 'notified'

    def log(self, own_token: str, level: int, message: str) -> None:
        if not own_token:
            return
        payLoad: typing.Dict[str, typing.Any] = {'token': own_token, 'level': level, 'message': message}
        self._doPost('log', payLoad)  # Ignores result...

    def test(self, master_token: str, actorType: typing.Optional[str]) -> bool:
        payLoad = {
            'type': actorType or types.MANAGED,
            'token': master_token,
        }
        return self._doPost('test', payLoad) == 'ok'


class UDSClientApi(UDSApi, metaclass=tools.Singleton):
    _session_id: str = ''
    _callback_url: str = ''

    def __init__(self) -> None:
        # Give a long timeout, so service can do its job with retries, etc..
        super().__init__('127.0.0.1:{}'.format(LISTEN_PORT), False, timeout=32)

        # Replace base url
        self._url = "https://{}/ui/".format(self._host)

    def _api_url(self, method: str) -> str:
        return self._url + method

    def post(
        self,
        method: str,  # i.e. 'initialize', 'ready', ....
        payLoad: typing.MutableMapping[str, typing.Any],
    ) -> typing.Any:
        return self._doPost(method=method, payload=payLoad, no_proxy=True)

    def register(self, callback_url: str) -> None:
        self._callback_url = callback_url
        payLoad = {'callback_url': callback_url}
        self.post('register', payLoad)

    def unregister(self, callback_url: str) -> None:
        payLoad = {'callback_url': callback_url}
        self.post('unregister', payLoad)
        self._callback_url = ''

    def login(self, username: str, sessionType: typing.Optional[str] = None) -> types.LoginResultInfoType:
        payLoad = {
            'username': username,
            'session_type': sessionType or UNKNOWN,
            'callback_url': self._callback_url,  # So we identify ourselves
        }
        result = self.post('login', payLoad)
        res = types.LoginResultInfoType(
            ip=result['ip'],
            hostname=result['hostname'],
            deadline=result['deadline'],
            max_idle=result['max_idle'],
            session_id=result['session_id'],
        )
        # Store session id for future use
        self._session_id = res.session_id or ''
        return res

    def logout(self, username: str, sessionType: typing.Optional[str] = None) -> None:
        payLoad = {
            'username': username,
            'session_type': sessionType or UNKNOWN,
            'callback_url': self._callback_url,  # So we identify ourselves
            'session_id': self._session_id,  # We now know the session id, provided on login
        }
        self.post('logout', payLoad)

    def ping(self) -> bool:
        return self.post('ping', {}) == 'pong'
