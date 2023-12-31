# -*- coding: utf-8 -*-
#
# Copyright (c) 2014-2019 Virtual Cable S.L.
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
# pylint: disable=invalid-name
import os
import typing
import configparser
import base64
import pickle  # nossec
import json

from udsactor import types, consts
from ..abc import ConfigReader


class LinuxConfigReader(ConfigReader):
    async def read(self) -> types.ActorConfiguration:
        try:
            cfg = configparser.ConfigParser()
            cfg.read(consts.CONFIGFILE)
            uds: configparser.SectionProxy = cfg['uds']

            cfgVersion = int(uds.get('config_version', '0'), 16)

            config: typing.Any = None
            data: typing.Any = None

            base64Config = uds.get('config', None)
            base64Data = uds.get('data', None)

            # Get old compat values and translate them to new ones if needed
            if cfgVersion == 0:  # Old version
                token = uds.get('master_token', None) or uds.get('own_token', None)
                # Extract data:
                config = (
                    pickle.loads(base64.b64decode(base64Config.encode()))  # nosec: file is restricted
                    if base64Config
                    else None
                )

                data = (
                    pickle.loads(base64.b64decode(base64Data.encode()))  # nosec: file is restricted
                    if base64Data
                    else None
                )
            elif cfgVersion == consts.CONFIG_VERSION:
                # New version has just token, and config and data are encoded as base64 but with json inside
                token = uds.get('token', None)
                config = json.loads(base64.b64decode(base64Config.encode())) if base64Config else None
                data = json.loads(base64.b64decode(base64Data.encode())) if base64Data else None
            else:  # No valid version!??
                token = None

            return types.ActorConfiguration(
                version=int(uds.get('config_version', '0'), 16),
                actorType=uds.get('type', types.ActorType.MANAGED),
                token=token,
                initialized=uds.getboolean('initialized', fallback=False),
                host=uds.get('host', ''),
                validateCertificate=uds.getboolean('validate', fallback=True),
                restrict_net=uds.get('restrict_net', None),
                pre_command=uds.get('pre_command', None),
                runonce_command=uds.get('runonce_command', None),
                post_command=uds.get('post_command', None),
                log_level=int(uds.get('log_level', '2')),
                config=config,
                data=data,
            )
        except Exception:
            return types.ActorConfiguration()

    async def write(self, config: types.ActorConfiguration) -> None:
        cfg = configparser.ConfigParser()
        cfg.add_section('uds')
        uds: configparser.SectionProxy = cfg['uds']

        # Store current config version
        uds['version'] = hex(config.version)[2:]  # Keep it as hex, so we can detect changes
        uds['host'] = config.host
        uds['validate'] = 'yes' if config.validateCertificate else 'no'
        uds['token_transformed'] = 'yes' if config.initialized else 'no'

        def writeIfValue(val, name):
            if val:
                uds[name] = val

        writeIfValue(config.actorType, 'type')
        writeIfValue(config.token, 'token')
        writeIfValue(config.restrict_net, 'restrict_net')
        writeIfValue(config.pre_command, 'pre_command')
        writeIfValue(config.post_command, 'post_command')
        writeIfValue(config.runonce_command, 'runonce_command')
        uds['log_level'] = str(config.log_level)
        if config.config:  # Special case, encoded as base64 from json dump
            uds['config'] = base64.b64encode(json.dumps(config.config.as_dict()).encode()).decode()

        if config.data:  # Special case, encoded as base64 from pickle dump
            uds['data'] = base64.b64encode(json.dumps(config.data).encode()).decode()

        # Ensures exists destination folder
        dirname = os.path.dirname(consts.CONFIGFILE)
        if not os.path.exists(dirname):
            os.mkdir(
                dirname, mode=0o700
            )  # Will create only if route to path already exists, for example, /etc (that must... :-))

        with open(consts.CONFIGFILE, 'w') as f:
            cfg.write(f)

        os.chmod(consts.CONFIGFILE, 0o0600)  # Ensure only readable by root

    async def scriptToInvokeOnLogin(self) -> str:
        return ''
