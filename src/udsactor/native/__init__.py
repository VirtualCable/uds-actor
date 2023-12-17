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
import sys

from . import abc
from udsactor import utils, types


class Manager(metaclass=utils.Singleton):
    cfgManager: abc.ConfigReader
    operations: abc.Operations
    runner: abc.Runner
    logger: typing.Any = None

    cfg: typing.Optional[types.ActorConfiguration] = None

    def __init__(self) -> None:
        # Import depending on platform, and set the proper values
        if sys.platform == 'win32':
            # pylint: disable=import-outside-toplevel
            from .windows import operations, config, service, log

            self.cfgManager = config.WindowsConfigReader()
            self.operations = operations.WindowsOperations()
            self.runner = service.WindowsRunner()
            self.logger = log.ServiceLogger()
        elif sys.platform == 'linux':
            # pylint: disable=import-outside-toplevel
            from .linux import operations, config, service

            self.cfgManager = config.LinuxConfigReader()
            self.operations = operations.LinuxOperations()
            self.runner = service.LinuxRunner()
            self.logger = None
        else:
            raise Exception('Unsupported platform')

    @staticmethod
    def instance() -> 'Manager':
        return Manager()  # As singleton, this will always return the same instance

    @property
    async def config(self) -> types.ActorConfiguration:
        '''Gets the configuration for this actor
        use with "xxx = await Platform.platform().config" to get the configuration
        '''
        if self.cfg is None:
            self.cfg = await self.cfgManager.read()

        return self.cfg

    async def clear_config(self) -> None:
        '''Clears the configuration for this actor
        So it gets reloaded on next access
        '''
        self.cfg = None
