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
import typing
import os
import tempfile
import platform


def _featureRequested(env_var: str) -> bool:
    env_var_name = env_var.upper().replace('-', '_')
    if env_var_name not in os.environ:
        # Look for temp file with that name, if it exists, its true
        if os.path.exists(os.path.join(tempfile.gettempdir(), env_var)):
            return True

    return os.getenv(env_var_name, 'false').lower() in ('true', 'yes', '1')


DEBUG: typing.Final[bool] = _featureRequested('uds-debug-on')

VERSION: typing.Final[str] = '4.0.0'
VERSION_FULL: typing.Final[str] = f'UDSActor {VERSION}'
BUILD: typing.Final[str] = '20231212'
SYSTEM: typing.Final[str] = platform.system().lower()

# File where configuration is stored
CONFIGFILE: typing.Final[str] = '/etc/udsactor/udsactor.cfg' if not DEBUG else 'udsactor.cfg'
CONFIG_VERSION: typing.Final[int] = 0x40000

ALLOW_NOADMIN: typing.Final[bool] = _featureRequested('uds-allow-noadmin')

# OK
OK: typing.Final[str] = 'ok'

# Default timeout
TIMEOUT: typing.Final[int] = 5  # 5 seconds is more than enought
WAIT_RETRY: typing.Final[int] = 5  # 5 seconds between retries
RETRIES: typing.Final[int] = 3  # 3 retries by default

# Unknow value
UNKNOWN: typing.Final[str] = 'unknown'

# Actor REST api paths
BASE_PUBLIC_REST_PATH: typing.Final[str] = '/actor'


CLIENT_SESSION_ID_FILE: typing.Final[str] = '/tmp/udsactor.session'


# For composing paths for public rest
def PUBLIC_REST_PATH(method: str) -> str:
    return f'{BASE_PUBLIC_REST_PATH}/{{auth_token}}/{method}'


BASE_PRIVATE_REST_PATH: typing.Final[str] = '/private'


# For composing paths for private rest
def PRIVATE_REST_PATH(method: str) -> str:
    return f'{BASE_PRIVATE_REST_PATH}/{method}'


# Private rest paths, allows connections without token if from localhost
PRIVATE_REST_LOGIN: typing.Final[str] = PRIVATE_REST_PATH('user_login')
PRIVATE_REST_LOGOUT: typing.Final[str] = PRIVATE_REST_PATH('user_logout')
PRIVATE_REST_LOG: typing.Final[str] = PRIVATE_REST_PATH('log')
PRIVATE_REST_WS: typing.Final[str] = PRIVATE_REST_PATH('ws')
VALID_PRIVATE_REST_PATHS: typing.Final[set[str]] = {
    PRIVATE_REST_LOGIN,
    PRIVATE_REST_LOGOUT,
    PRIVATE_REST_LOG,
    PRIVATE_REST_WS,
}

# 128Kb maximum size for client requests
CLIENT_MAX_SIZE: typing.Final[int] = 128 * 1024

# Default listen port
LISTEN_PORT: typing.Final[int] = 43910

# Stats send interval
STATS_INTERVAL: typing.Final[int] = 60  # 60 seconds

# SSL Related
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
