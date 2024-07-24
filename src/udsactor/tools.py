# -*- coding: utf-8 -*-
#
# Copyright (c) 2019 Virtual Cable S.L.
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
@author: Adolfo Gómez, dkmaster at dkmon dot com
'''
import threading
import ipaddress
import time
import typing
import collections.abc
import functools
import ssl

import urllib3
import urllib3.exceptions
import requests
import requests.adapters

from udsactor.version import VERSION, BUILD


SECURE_CIPHERS: typing.Final[str] = (
    'TLS_AES_256_GCM_SHA384'
    ':TLS_CHACHA20_POLY1305_SHA256'
    ':TLS_AES_128_GCM_SHA256'
    ':ECDHE-RSA-AES256-GCM-SHA384'
    ':ECDHE-RSA-AES128-GCM-SHA256'
    ':ECDHE-RSA-CHACHA20-POLY1305'
    ':ECDHE-ECDSA-AES128-GCM-SHA256'
    ':ECDHE-ECDSA-AES256-GCM-SHA384'
    ':ECDHE-ECDSA-CHACHA20-POLY1305'
)


if typing.TYPE_CHECKING:
    from udsactor.types import InterfaceInfoType

# Simple cache for n seconds (default = 30) decorator
def cache(seconds: int = 30) -> collections.abc.Callable:
    '''
    Simple cache for n seconds (default = 30) decorator
    '''
    def decorator(func) -> collections.abc.Callable:
        @functools.wraps(func)
        def wrapper(*args, **kwargs) -> typing.Any:
            if not hasattr(wrapper, 'cache'):
                wrapper.cache = {}  # type: ignore
            cache = wrapper.cache  # type: ignore

            # Compose a key for the cache
            key = '{}:{}'.format(args, kwargs)
            if key in cache:
                if time.time() - cache[key][0] < seconds:
                    return cache[key][1]
            
            # Call the function
            result = func(*args, **kwargs)
            cache[key] = (time.time(), result)
            return result

        return wrapper

    return decorator


# Simple sub-script exectution thread
class ScriptExecutorThread(threading.Thread):
    def __init__(self, script: str) -> None:
        super(ScriptExecutorThread, self).__init__()
        self.script = script

    def run(self) -> None:
        from udsactor.log import logger

        try:
            logger.debug('Executing script: {}'.format(self.script))
            exec(
                self.script, globals(), None
            )  # nosec: exec is fine, it's a "trusted" script
        except Exception as e:
            logger.error('Error executing script: {}'.format(e))
            logger.exception()


class Singleton(type):
    '''
    Metaclass for singleton pattern
    Usage:

    class MyClass(metaclass=Singleton):
        ...
    '''

    _instance: typing.Optional[typing.Any]

    # We use __init__ so we customise the created class from this metaclass
    def __init__(self, *args, **kwargs) -> None:
        self._instance = None
        super().__init__(*args, **kwargs)

    def __call__(self, *args, **kwargs) -> typing.Any:
        if self._instance is None:
            self._instance = super().__call__(*args, **kwargs)
        return self._instance


# Convert "X.X.X.X/X" to ipaddress.IPv4Network
def strToNoIPV4Network(
    net: typing.Optional[str],
) -> typing.Optional[ipaddress.IPv4Network]:
    if not net:  # Empty or None
        return None
    try:
        return ipaddress.IPv4Interface(net).network
    except Exception:
        return None


def validNetworkCards(
    net: typing.Optional[str], cards: typing.Iterable['InterfaceInfoType']
) -> list['InterfaceInfoType']:
    try:
        subnet = strToNoIPV4Network(net)
    except Exception as e:
        subnet = None

    if subnet is None:
        return list(cards)

    def isValid(ip: str, subnet: ipaddress.IPv4Network) -> bool:
        if not ip:
            return False
        try:
            return ipaddress.IPv4Address(ip) in subnet
        except Exception:
            return False

    return [c for c in cards if isValid(c.ip, subnet)]

def create_client_sslcontext(verify: bool = True) -> ssl.SSLContext:
    """
    Creates a SSLContext for client connections.

    Args:
        verify: If True, the server certificate will be verified. (Default: True)

    Returns:
        A SSLContext object.
    """
    ssl_context = ssl.create_default_context(purpose=ssl.Purpose.SERVER_AUTH)
    if not verify:
        ssl_context.check_hostname = False
        ssl_context.verify_mode = ssl.VerifyMode.CERT_NONE

    # Disable TLS1.0 and TLS1.1, SSLv2 and SSLv3 are disabled by default
    # Next line is deprecated in Python 3.7
    ssl_context.minimum_version = ssl.TLSVersion.TLSv1_2
    ssl_context.set_ciphers(SECURE_CIPHERS)
    ssl_context.maximum_version = ssl.TLSVersion.MAXIMUM_SUPPORTED

    return ssl_context

def secure_requests_session(*, verify: typing.Union[str, bool] = True) -> 'requests.Session':
    '''
    Generates a requests.Session object with a custom adapter that uses a custom SSLContext.
    This is intended to be used for requests that need to be secure, but not necessarily verified.
    Removes the support for TLS1.0 and TLS1.1, and disables SSLv2 and SSLv3. (done in @createClientSslContext)

    Args:
        verify: If True, the server certificate will be verified. (Default: True)

    Returns:
        A requests.Session object.
    '''

    # Copy verify value
    lverify = verify

    # Disable warnings from urllib for insecure requests
    # Note that although this is done globaly, on some circunstances, may be overriden later
    # This will ensure that we do not get warnings about self signed certificates
    urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

    class UDSHTTPAdapter(requests.adapters.HTTPAdapter):
        ssl_context: ssl.SSLContext
        def init_poolmanager(self, *args: typing.Any, **kwargs: typing.Any) -> None:
            self.ssl_context = kwargs["ssl_context"] = create_client_sslcontext(verify=verify is True)

            # See urllib3.poolmanager.SSL_KEYWORDS for all available keys.
            return super().init_poolmanager(*args, **kwargs)  # type: ignore

        def cert_verify(self, conn: typing.Any, url: typing.Any, verify: 'str|bool', cert: typing.Any) -> None:
            """Verify a SSL certificate. This method should not be called from user
            code, and is only exposed for use when subclassing the HTTPAdapter class
            """

            # If lverify is an string, use it even if verify is False
            # if not, use verify value
            if not isinstance(verify, str):
                verify = lverify

            # 2.32  version of requests, broke the hability to override the ssl_context
            # Please, ensure that you are using a version of requests that is compatible with this code (2.32.3) or newer
            # And this way, our ssl_context is not used, so we need to override it again to ensure that our ssl_context is used
            # if 'conn_kw' in conn.__dict__:
            #     conn_kw = conn.__dict__['conn_kw']
            #     conn_kw['ssl_context'] = self.ssl_context

            super().cert_verify(conn, url, verify, cert)  # type: ignore

    session = requests.Session()
    session.mount("https://", UDSHTTPAdapter())

    # Add user agent header to session
    session.headers.update({"User-Agent": f'UDSActor/{VERSION} (Build {BUILD})'})

    return session
