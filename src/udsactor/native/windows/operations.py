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
import asyncio
import collections.abc
import ctypes
import logging
import os
import subprocess
import typing
from ctypes.wintypes import DWORD, LPCWSTR

import ntsecuritycon
import win32api
import win32com.client
import win32con
import win32net
import win32security
from win32com.shell import shell
import pythoncom

from udsactor import types

from ..abc import Operations

logger = logging.getLogger(__name__)

# Reboot flags
EWX_LOGOFF: typing.Final[int] = 0x00000000
EWX_SHUTDOWN: typing.Final[int] = 0x00000001
EWX_REBOOT: typing.Final[int] = 0x00000002
EWX_FORCE: typing.Final[int] = 0x00000004
EWX_POWEROFF: typing.Final[int] = 0x00000008
EWX_FORCEIFHUNG: typing.Final[int] = 0x00000010

# Join domain flags
NETSETUP_JOIN_DOMAIN: typing.Final[int] = 0x00000001
NETSETUP_ACCT_CREATE: typing.Final[int] = 0x00000002
NETSETUP_ACCT_DELETE: typing.Final[int] = 0x00000004
NETSETUP_WIN9X_UPGRADE: typing.Final[int] = 0x00000010
NETSETUP_DOMAIN_JOIN_IF_JOINED: typing.Final[int] = 0x00000020
NETSETUP_JOIN_UNSECURE: typing.Final[int] = 0x00000040
NETSETUP_MACHINE_PWD_PASSED: typing.Final[int] = 0x00000080
NETSETUP_JOIN_WITH_NEW_NAME: typing.Final[int] = 0x00000400
NETSETUP_DEFER_SPN_SET: typing.Final[int] = 0x1000000


class WindowsOperations(Operations):
    async def IsUserAnAdmin(self) -> bool:
        return shell.IsUserAnAdmin()

    async def getComputerName(self) -> str:
        return win32api.GetComputerNameEx(win32con.ComputerNamePhysicalDnsHostname)

    async def getNetworkInfo(self) -> list[types.InterfaceInfo]:
        def fetch_from_wmi() -> list[types.InterfaceInfo]:
            result: list[types.InterfaceInfo] = []
            # Ensure coinitialize is called. Needed because "run_in_executor" will create a new thread
            pythoncom.CoInitialize()
            obj = win32com.client.Dispatch("WbemScripting.SWbemLocator")
            wmobj = obj.ConnectServer("localhost", "root\\cimv2")
            adapters = wmobj.ExecQuery("Select * from Win32_NetworkAdapterConfiguration where IpEnabled=True")
            for obj in adapters:
                try:
                    for ip in obj.IPAddress:
                        if ':' in ip:  # Is IPV6, skip this
                            continue
                        if (
                            ip is None or ip == '' or ip.startswith('169.254') or ip.startswith('0.')
                        ):  # If single link ip, or no ip
                            continue
                        result.append(types.InterfaceInfo(name=obj.Caption, mac=obj.MACAddress, ip=ip))
                except Exception:
                    pass
            return result

        # wmi can take some time, so we run it in a thread
        return await asyncio.get_running_loop().run_in_executor(None, fetch_from_wmi)

    async def getDomainName(self) -> str:
        '''
        Will return the domain name if we belong a domain, else None
        (if part of a network group, will also return None)
        '''
        # Status:
        # 0 = Unknown
        # 1 = Unjoined
        # 2 = Workgroup
        # 3 = Domain
        domain, status = win32net.NetGetJoinInformation()
        if status != 3:
            domain = ''

        return domain

    async def getOSName(self) -> str:
        verinfo = self._getWindowsVersion()
        return f'Windows {verinfo[0]}.{verinfo[1]}'

    async def getOSVersion(self) -> str:
        verinfo = self._getWindowsVersion()
        # Remove platform id i
        return f'Windows-{verinfo[0]}.{verinfo[1]} Build {verinfo[2]} ({verinfo[3]}))'

    async def reboot(self, flags: int = 0) -> None:
        flags = EWX_FORCEIFHUNG | EWX_REBOOT if flags == 0 else flags
        hproc = win32api.GetCurrentProcess()
        htok = win32security.OpenProcessToken(
            hproc, win32security.TOKEN_ADJUST_PRIVILEGES | win32security.TOKEN_QUERY
        )
        privs = (
            (
                win32security.LookupPrivilegeValue(None, win32security.SE_SHUTDOWN_NAME),  # type: ignore
                win32security.SE_PRIVILEGE_ENABLED,
            ),
        )
        win32security.AdjustTokenPrivileges(htok, 0, privs)  # type: ignore
        win32api.ExitWindowsEx(flags, 0)

    async def loggoff(self) -> None:
        win32api.ExitWindowsEx(EWX_LOGOFF)

    async def renameComputer(self, newName: str) -> bool:
        '''
        Changes the computer name
        Returns True if reboot needed
        '''
        # Needs admin privileges to work
        if (
            ctypes.windll.kernel32.SetComputerNameExW(  # type: ignore
                DWORD(win32con.ComputerNamePhysicalDnsHostname), LPCWSTR(newName)
            )
            == 0
        ):  # @UndefinedVariable
            # win32api.FormatMessage -> returns error string
            # win32api.GetLastError -> returns error code
            # (just put this comment here to remember to log this when logger is available)
            error = self._getErrorMessage()
            computerName = win32api.GetComputerNameEx(win32con.ComputerNamePhysicalDnsHostname)
            raise Exception('Error renaming computer from {} to {}: {}'.format(computerName, newName, error))
        return True

    async def joinDomain(self, **kwargs: typing.Any) -> None:
        def innerJoinDomain() -> None:
            domain = kwargs.get('domain', None)
            ou = kwargs.get('ou', None)
            account = kwargs.get('account', None)
            password = kwargs.get('password', None)
            executeInOneStep = kwargs.get('executeInOneStep', False)

            if domain is None or account is None or password is None:
                raise Exception('Domain, account and password are mandatory to join a domain')

            # If account do not have domain, include it
            if '@' not in account and '\\' not in account:
                if '.' in domain:
                    account = account + '@' + domain
                else:
                    account = domain + '\\' + account

            # Do log
            iflags: int = NETSETUP_ACCT_CREATE | NETSETUP_DOMAIN_JOIN_IF_JOINED | NETSETUP_JOIN_DOMAIN

            if executeInOneStep:
                iflags |= NETSETUP_JOIN_WITH_NEW_NAME

            flags: DWORD = DWORD(iflags)

            lpDomain = LPCWSTR(domain)

            # Must be in format "ou=.., ..., dc=...,"
            lpOu = LPCWSTR(ou) if ou else None
            lpAccount = LPCWSTR(account)
            lpPassword = LPCWSTR(password)

            res = ctypes.windll.netapi32.NetJoinDomain(  # type: ignore
                None, lpDomain, lpOu, lpAccount, lpPassword, flags
            )
            # Machine found in another ou, use it and warn this on log
            if res == 2224:
                flags = DWORD(NETSETUP_DOMAIN_JOIN_IF_JOINED | NETSETUP_JOIN_DOMAIN)
                res = ctypes.windll.netapi32.NetJoinDomain(  # type: ignore
                    None, lpDomain, None, lpAccount, lpPassword, flags
                )
            if res:
                # Log the error
                if res == 1355:
                    error = "DC Is not reachable"
                else:
                    error = self._getErrorMessage(res)
                logger.error('Error joining domain: {}, {}'.format(error, res))
                raise Exception(
                    'Error joining domain {}, with credentials {}/*****{}: {}, {}'.format(
                        domain,
                        account,
                        ', under OU {}'.format(ou) if ou is not None else '',
                        res,
                        error,
                    )
                )

        # Join domain can take some time, so we run it in a thread
        await asyncio.get_running_loop().run_in_executor(None, innerJoinDomain)

    async def changeUserPassword(self, user: str, oldPassword: str, newPassword: str) -> None:
        # lpUser = LPCWSTR(user)
        # lpOldPassword = LPCWSTR(oldPassword)
        # lpNewPassword = LPCWSTR(newPassword)

        # res = ctypes.windll.netapi32.NetUserChangePassword(None, lpUser, lpOldPassword, lpNewPassword)
        # Try to set new password "a las bravas", ignoring old one. This will not work with domain users
        res = win32net.NetUserSetInfo(None, user, 1003, {'password': newPassword})  # type: ignore

        if res:
            # Log the error, and raise exception to parent
            error = self._getErrorMessage(res)
            raise Exception('Error changing password for user {}: {} {}'.format(user, res, error))

    async def initIdleDuration(self, atLeastSeconds: int) -> None:
        pass

    async def getIdleDuration(self) -> float:
        class LASTINPUTINFO(ctypes.Structure):  # pylint: disable=too-few-public-methods
            _fields_ = [
                ('cbSize', ctypes.c_uint),
                ('dwTime', ctypes.c_uint),
            ]

        try:
            lastInputInfo = LASTINPUTINFO()
            lastInputInfo.cbSize = ctypes.sizeof(
                lastInputInfo
            )  # pylint: disable=attribute-defined-outside-init
            if ctypes.windll.user32.GetLastInputInfo(ctypes.byref(lastInputInfo)) == 0:  # type: ignore
                return 0
            current = ctypes.c_uint(ctypes.windll.kernel32.GetTickCount()).value  # type: ignore
            if current < lastInputInfo.dwTime:
                current += (
                    4294967296  # If current has "rolled" to zero, adjust it so it is greater than lastInputInfo
                )
            millis = current - lastInputInfo.dwTime  # @UndefinedVariable
            return millis / 1000.0
        except Exception as e:
            logger.error('Getting idle duration: {}'.format(e))
            return 0

    async def getCurrentUser(self) -> str:
        return win32api.GetUserName()

    async def getSessionType(self) -> str:
        '''
        Known values:
        * Unknown -> No SESSIONNAME environment variable
        * Console -> Local session
        *  RDP-Tcp#[0-9]+ -> RDP Session
        '''
        return os.environ.get('SESSIONNAME', 'unknown')

    async def forceTimeSync(self) -> None:
        try:
            # subprocess.call([r'c:\WINDOWS\System32\w32tm.exe', ' /resync'])  # , '/rediscover'])
            p = await asyncio.create_subprocess_exec(
                r'c:\WINDOWS\System32\w32tm.exe',
                '/resync',
                stdin=asyncio.subprocess.DEVNULL,
                stdout=asyncio.subprocess.DEVNULL,
                stderr=asyncio.subprocess.DEVNULL,
            )  # , '/rediscover'])
            await p.wait()
        except Exception as e:
            logger.error('Error invoking time sync command: %s', e)

    async def protectFileForOwnerOnly(self, filepath: str) -> None:
        try:
            user, domain, _type = win32security.LookupAccountName('', await self.getCurrentUser())

            secDescriptor = win32security.GetFileSecurity(filepath, win32security.DACL_SECURITY_INFORMATION)
            dACL = secDescriptor.GetSecurityDescriptorDacl()
            dACL.AddAccessAllowedAce(win32security.ACL_REVISION, ntsecuritycon.FILE_ALL_ACCESS, user)
            secDescriptor.SetSecurityDescriptorDacl(1, dACL, 0)
            win32security.SetFileSecurity(filepath, win32security.DACL_SECURITY_INFORMATION, secDescriptor)
        except Exception as e:
            logger.error('Error protecting file %s: %s', filepath, e)

    async def setTitle(self, title: str) -> None:
        win32api.SetConsoleTitle(title)

    # High level operations
    async def hloJoinDomain(self, name: str, custom: collections.abc.Mapping[str, typing.Any]) -> bool:
        '''
        Joins domain with given name and custom parameters

        Returns true if reboot is needed, false otherwise
        '''
        versionData = self._getWindowsVersion()
        versionInt = versionData[0] * 10 + versionData[1]

        # Extract custom data
        domain = custom.get('domain', '')
        ou = custom.get('ou', '')
        account = custom.get('account', '')
        password = custom.get('password', '')

        logger.debug(
            'Starting joining domain {} with name {} (detected operating version: {})'.format(
                domain, name, versionData
            )
        )
        # Accepts one step joinDomain, also remember XP is no more supported by
        # microsoft, but this also must works with it because will do a "multi
        # step" join
        if versionInt >= 60:
            return await self.oneStepJoin(name, domain, ou, account, password)

        logger.info('Using multiple step join because os is not windows vista or higher')
        return await self.multiStepJoin(name, domain, ou, account, password)

    # Custom, private methods
    async def oneStepJoin(
        self, name: str, domain: str, ou: str, account: str, password: str
    ) -> bool:  # pylint: disable=too-many-arguments
        '''
        Ejecutes the join domain in exactly one step
        '''
        currName = await self.getComputerName()
        # If name is desired, simply execute multiStepJoin, because computer
        # name will not change
        if currName.lower() == name.lower():
            await self.multiStepJoin(name, domain, ou, account, password)
            return False

        await self.renameComputer(name)
        logger.debug('Computer renamed to {} without reboot'.format(name))
        await self.joinDomain(domain=domain, ou=ou, account=account, password=password, executeInOneStep=True)
        logger.debug('Requested join domain {} without errors'.format(domain))
        return True

    async def multiStepJoin(self, name: str, domain: str, ou: str, account: str, password: str) -> bool:
        """Joins a domain in two steps, first renaming computer, and then joining domain
        Returns True if reboot is needed (that is, an step has been executed), False otherwise
        """
        currName = await self.getComputerName()
        if currName.lower() == name.lower():
            currDomain = await self.getDomainName()
            if currDomain:
                # logger.debug('Name: "{}" vs "{}", Domain: "{}" vs "{}"'.format(currName.lower(), name.lower(), currDomain.lower(), domain.lower()))
                logger.debug('Machine {} is part of domain {}'.format(name, domain))
                return False
            else:
                logger.info('Joining domain {}'.format(domain))
                await self.joinDomain(
                    domain=domain, ou=ou, account=account, password=password, executeInOneStep=False
                )
                return True

        await self.renameComputer(name)
        logger.info('Activating new name {}'.format(name))
        return True

    def _getErrorMessage(self, resultCode: int = 0) -> str:
        # sys_fs_enc = sys.getfilesystemencoding() or 'mbcs'
        msg = win32api.FormatMessage(resultCode)
        return msg

    def _getWindowsVersion(self) -> tuple[int, int, int, int, str]:
        return win32api.GetVersionEx()

    # def writeToPipe(pipeName: str, bytesPayload: bytes, waitForResponse: bool) -> typing.Optional[bytes]:
    #     # (str, bytes, bool) -> Optional[bytes]
    #     try:
    #         with open(pipeName, 'r+b', 0) as f:
    #             f.write(bytesPayload)
    #             # f.seek(0)  # As recommended on intenet, but seems to work fin without thos
    #             if waitForResponse:
    #                 return f.read()
    #         return b'ok'
    #     except Exception:
    #         return None
