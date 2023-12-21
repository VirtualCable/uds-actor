# -*- coding: utf-8 -*-
#
# Copyright (c) 2024 Virtual Cable S.L.U.
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
'''

import logging
import typing
import collections.abc
import threading
import asyncio
import sys

import win32serviceutil
import win32service
import win32net
import pythoncom
import servicemanager
import win32security

from udsactor import types, consts
from udsactor.server import UDSActorServer

logger = logging.getLogger(__name__)


class WindowsUDSActorServer(UDSActorServer):
    async def preconnect(self, *, data: types.PreconnectRequest) -> None:
        logger.debug('Pre connect invoked')

        if data.protocol == 'rdp':  # If connection is not using rdp, skip adding user
            # We cast None to str so mypy does not complains :)
            groupName = win32security.LookupAccountSid(
                typing.cast(str, None), win32security.GetBinarySid(consts.REMOTE_USERS_SID)
            )[0]

            useraAlreadyInGroup = False
            resumeHandle = 0
            # Note that this loop is fast enough, so we can do it syncronously
            while True:
                users, _, resumeHandle = win32net.NetLocalGroupGetMembers(
                    typing.cast(str, None), groupName, 1, resumeHandle, 32768
                )[
                    :3
                ]  # In fact, NetLocalGroupGetMembers returns 3 values, but type checker thinks it should be 4 :(
                if data.username.lower() in [u['name'].lower() for u in users]:
                    useraAlreadyInGroup = True
                    break
                if resumeHandle == 0:
                    break

            if not useraAlreadyInGroup:
                logger.debug('User not in group, adding it')
                try:
                    userSSID = win32security.LookupAccountName(None, data.username)[0]
                    # Cast to typing.any is due to type checker not knowing that the parameter is a list of dicts
                    win32net.NetLocalGroupAddMembers(
                        typing.cast(str, None), groupName, 0, typing.cast(typing.Any, [{'sid': userSSID}])
                    )
                except Exception as e:
                    logger.error('Exception adding user to Remote Desktop Users: {}'.format(e))
            else:
                self._user = None
                logger.debug('User {} already in group'.format(data.username))

        return await super().preconnect(data=data)
