# -*- coding: utf-8 -*-
#
# Copyright (c) 2022 Virtual Cable S.L.U.
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
import typing
import socket
import logging
import random
import sys
import unittest

from udsactor import consts, native, types

logger = logging.getLogger(__name__)


def get_free_port(ipv6: bool = False) -> int:
    '''Returns a free port in the system'''
    if not ipv6:
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    else:
        s = socket.socket(socket.AF_INET6, socket.SOCK_STREAM)
    s.bind(('', 0))
    port = s.getsockname()[1]
    s.close()
    return port


def rnd_string_for_test(length: int = 32) -> str:
    return ''.join(random.choice('0123456789ABCDEF') for i in range(length))  # nosec: testing purposes only


def public_rest_path(
    method: str, *, server: str = f'https://localhost:{consts.LISTEN_PORT}', token: typing.Optional[str] = None
) -> str:
    return server.strip('/') + consts.PUBLIC_REST_PATH(method).replace(
        '{auth_token}', token or consts.OWN_AUTH_TOKEN
    )


_previous_cfg: typing.Optional[types.ActorConfiguration] = None


def set_testing_cfg(cfg: typing.Optional[types.ActorConfiguration]) -> None:
    global _previous_cfg

    class NoReaderWriter(native.abc.ConfigReader):
        cfg: types.ActorConfiguration

        def __init__(self, cfg: types.ActorConfiguration):
            self.cfg = cfg

        async def read(self) -> types.ActorConfiguration:
            return self.cfg

        async def write(self, cfg: types.ActorConfiguration) -> None:
            self.cfg = cfg

        async def scriptToInvokeOnLogin(self) -> str:
            return ''

    # Override "save" config, platform provided config, etc..
    p = native.Manager.instance()
    # Ensure own is used
    if _previous_cfg is None and p.cfg is not None:
        _previous_cfg = p.cfg
    if cfg is not None:
        p.cfgManager = NoReaderWriter(cfg)
    elif _previous_cfg is not None:
        p.cfgManager = NoReaderWriter(_previous_cfg)

    p.cfg = None  # Cleans up cfg, so next read will reload it


def skip_if_not(platforms: 'list[str] | str') -> None:
    platforms = [platforms] if isinstance(platforms, str) else platforms

    if sys.platform not in platforms:
        raise unittest.SkipTest(f'Test skipped on platform {sys.platform} (desired: {",".join(platforms)})')
