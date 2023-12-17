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
import typing
import json
import base64

import winreg as wreg
import win32security

from udsactor import types
from ..abc import ConfigReader

PATH: typing.Final[str] = 'Software\\UDSActor\\Config'  # Moved to "Config" on 4.0 to ensure that 3.x data (not compatible) is not used
BASEKEY: typing.Final[str] = wreg.HKEY_LOCAL_MACHINE  # type: ignore


def fixRegistryPermissions(handle) -> None:
    # Fix permissions so users can't read this key
    v = win32security.GetSecurityInfo(
        handle, win32security.SE_REGISTRY_KEY, win32security.DACL_SECURITY_INFORMATION
    )
    dacl = v.GetSecurityDescriptorDacl()
    n = 0
    # Remove all normal users access permissions to the registry key
    while n < dacl.GetAceCount():
        if str(dacl.GetAce(n)[2]) == 'PySID:S-1-5-32-545':  # Whell known Users SID
            dacl.DeleteAce(n)
        else:
            n += 1
    win32security.SetSecurityInfo(
        handle,
        win32security.SE_REGISTRY_KEY,
        win32security.DACL_SECURITY_INFORMATION | win32security.PROTECTED_DACL_SECURITY_INFORMATION,
        None,
        None,
        dacl,
        None,
    )


class WindowsConfigReader(ConfigReader):
    async def read(self) -> types.ActorConfiguration:
        try:
            key = wreg.OpenKey(BASEKEY, PATH, 0, wreg.KEY_QUERY_VALUE)
            data, _ = wreg.QueryValueEx(key, '')
            wreg.CloseKey(key)
            data = base64.b64decode(data).decode('utf8')
            data = json.loads(data)
            return types.ActorConfiguration.fromDict(data)
        except Exception:
            return types.ActorConfiguration()

    async def write(self, config: types.ActorConfiguration) -> None:
        try:
            key = wreg.OpenKey(BASEKEY, PATH, 0, wreg.KEY_ALL_ACCESS)
        except Exception:
            key = wreg.CreateKeyEx(BASEKEY, PATH, 0, wreg.KEY_ALL_ACCESS)

        fixRegistryPermissions(key.handle)  # type: ignore
        data = json.dumps(config.asDict())
        data = base64.b64encode(data.encode('utf8')).decode('utf8')

        wreg.SetValueEx(key, "", 0, wreg.REG_BINARY, data)  # type: ignore
        wreg.CloseKey(key)

    async def scriptToInvokeOnLogin(self) -> str:
        try:
            key = wreg.OpenKey(BASEKEY, PATH, 0, wreg.KEY_QUERY_VALUE)
            try:
                data, _ = wreg.QueryValueEx(key, 'logonScript')
            except Exception:
                data = ''
            wreg.CloseKey(key)
        except Exception:
            data = ''

        return data
