# -*- coding: utf-8 -*-
#
# Copyright (c) 2014-2023 Virtual Cable S.L.
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
@author: Alexander Burmatov,  thatman at altlinux dot org
'''
# pylint: disable=invalid-name
import configparser
import platform
import socket
import fcntl  # Only available on Linux. Expect complains if edited from windows
import os
import asyncio
import struct
import array
import typing
import logging

try:
    from setproctitle import setproctitle  # pyright: ignore[reportMissingImports,reportUnknownVariable,reportUnknownVariableType]
except ImportError:  # Platform may not include prctl, so in case it's not available, we let the "name" as is

    def setproctitle(title: str) -> None:
        pass


from udsactor import types
from udsactor.native.abc import Operations

from .renamer import rename
from . import xss

logger = logging.getLogger(__name__)


class LinuxOperations(Operations):
    @staticmethod
    async def get_interfaces() -> list[str]:
        '''
        Returns a list of interfaces names coded in utf-8
        '''
        max_possible = 128  # arbitrary. raise if needed.
        space = max_possible * 16
        if platform.architecture()[0] == '32bit':
            offset, length = 32, 32
        elif platform.architecture()[0] == '64bit':
            offset, length = 16, 40
        else:
            raise OSError('Unknown arquitecture {0}'.format(platform.architecture()[0]))

        s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
        names = array.array(str('B'), b'\0' * space)
        outbytes = struct.unpack(
            'iL',
            fcntl.ioctl(
                s.fileno(),
                0x8912,  # SIOCGIFCONF
                struct.pack('iL', space, names.buffer_info()[0]),
            ),
        )[0]
        namestr = names.tobytes()
        # return namestr, outbytes
        return [namestr[i : i + offset].split(b'\0', 1)[0].decode('utf-8') for i in range(0, outbytes, length)]

    @staticmethod
    async def get_ip_mac(
        ifname: str,
    ) -> tuple[typing.Optional[str], typing.Optional[str]]:
        def _get_mac_address(ifname: str) -> typing.Optional[str]:
            '''
            Returns the mac address of an interface
            Mac is returned as unicode utf-8 encoded
            '''
            ifname_bytes = ifname.encode('utf-8')
            try:
                s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
                info = bytearray(fcntl.ioctl(s.fileno(), 0x8927, struct.pack(str('256s'), ifname_bytes[:15])))
                return str(''.join(['%02x:' % char for char in info[18:24]])[:-1]).upper()
            except Exception:
                return None

        def _get_ip_address(ifname: str) -> typing.Optional[str]:
            '''
            Returns the ip address of an interface
            Ip is returned as unicode utf-8 encoded
            '''
            ifname_bytes = ifname.encode('utf-8')
            try:
                s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
                return str(
                    socket.inet_ntoa(
                        fcntl.ioctl(
                            s.fileno(),
                            0x8915,  # SIOCGIFADDR
                            struct.pack(str('256s'), ifname_bytes[:15]),
                        )[20:24]
                    )
                )
            except Exception:
                return None

        ip, mac = _get_ip_address(ifname), _get_mac_address(ifname)
        return (ip, mac)

    async def is_user_admin(self) -> bool:
        return os.getuid() == 0

    async def computer_name(self) -> str:
        '''
        Returns computer name, with no domain
        '''
        return socket.gethostname().split('.')[0]

    async def list_interfaces(self) -> list[types.InterfaceInfo]:
        result: list[types.InterfaceInfo] = []
        for ifname in await LinuxOperations.get_interfaces():
            ip, mac = await LinuxOperations.get_ip_mac(ifname)
            if (
                mac != '00:00:00:00:00:00' and mac and ip and ip.startswith('169.254') is False
            ):  # Skips local interfaces & interfaces with no dhcp IPs
                result.append(types.InterfaceInfo(name=ifname, mac=mac, ip=ip))
        return result

    async def domain_name(self) -> str:
        return (socket.getfqdn().split('.', 1) + [''])[1]

    async def os_name(self) -> str:
        try:
            with open('/etc/os-release', 'r') as f:
                data = f.read()
            cfg = configparser.ConfigParser()
            cfg.read_string('[os]\n' + data)
            return cfg['os'].get('id', 'unknown').replace('"', '')
        except Exception:
            return 'unknown'

    async def os_version(self) -> str:
        return 'Linux ' + await self.os_name()

    async def reboot(self, flags: int = 0) -> None:
        '''
        Simple reboot using os command
        '''
        try:
            p = await asyncio.create_subprocess_shell('/sbin/shutdown now -r')
            await p.wait()
        except Exception as e:
            logger.error('Error rebooting: %s', e)

    async def loggoff(self) -> None:
        '''
        Simple logoff using os command
        '''
        try:
            p = await asyncio.create_subprocess_shell(f'/usr/bin/pkill -u {os.environ["USER"]}')
            await p.wait()
        except Exception as e:
            logger.error('Error killing user processes: %s', e)
        # subprocess.call(['/sbin/shutdown', 'now', '-r'])
        # subprocess.call(['/usr/bin/systemctl', 'reboot', '-i'])

    async def rename_computer(self, newName: str) -> bool:
        '''
        Changes the computer name
        Returns True if reboot needed
        '''
        await rename(self, newName)
        return True  # Always reboot right now. Not much slower but much more convenient

    async def join_domain(self, **kwargs: typing.Any) -> None:
        custom = kwargs.get('custom', None)

        if not custom:
            logger.error('Error joining domain: no customization data provided')
            return

        # Read parameters from custom data
        domain: str = custom.get('domain', '')
        # If no domain, nothing to do !?!
        if not domain:
            logger.error('Error joining domain: no domain provided')
            return
        ou: str = custom.get('ou', '')
        account: str = custom.get('account', '')
        password: str = custom.get('password', '')
        client_software: str = custom.get('client_software', '')
        server_software: str = custom.get('server_software', '')
        membership_software: str = custom.get('membership_software', '')
        ssl: bool = custom.get('ssl', False)
        automatic_id_mapping: bool = custom.get('automatic_id_mapping', False)

        if server_software == 'ipa':
            try:
                hostname = (await self.computer_name()).lower() + '.' + domain
                p = await asyncio.create_subprocess_shell(f'hostnamectl set-hostname {hostname}')
                await p.wait()
            except Exception as e:
                logger.error(f'Error set hostname for freeeipa: {e}')
        try:
            command: list[str] = ['realm', 'join', '-U', account]
            # command = f'realm join -U {account} '
            if client_software and client_software != 'automatically':
                command.append(f'--client-software={client_software}')
            if server_software:
                command.append(f'--server-software={server_software}')
            if membership_software and membership_software != 'automatically':
                command.append(f'--membership-software={membership_software}')
            if ou and server_software != 'ipa':
                command.append(f'--computer-ou={ou}')
            if ssl:
                command.append('--use-ldaps')
            if not automatic_id_mapping:
                command.append('--automatic-id-mapping=no')
            command.append(domain)
            p = await asyncio.create_subprocess_shell(' '.join(command), stdin=asyncio.subprocess.PIPE)
            await p.communicate(password.encode())
        except Exception as e:
            logger.error(f'Error join machine to domain {domain}: {e}')

    async def change_user_password(self, user: str, oldPassword: str, newPassword: str) -> None:
        '''
        Simple password change for user on linux
        '''
        try:
            p = await asyncio.create_subprocess_exec(
                '/usr/bin/passwd', user, stdin=asyncio.subprocess.PIPE, stdout=asyncio.subprocess.PIPE
            )
            await p.communicate(f'{oldPassword}\n{newPassword}\n{newPassword}\n'.encode('utf-8'))
        except Exception as e:
            logger.error('Error changing password: %s', e)

    async def init_idle_duration(self, atLeastSeconds: int) -> None:
        await xss.initIdleDuration(atLeastSeconds)

    async def current_idle(self) -> float:
        return await xss.getIdleDuration()

    async def whoami(self) -> str:
        '''
        Returns current logged in user
        '''
        return os.getlogin()

    async def session_type(self) -> str:
        '''
        Known values:
        * Unknown -> No XDG_SESSION_TYPE environment variable
        * xrdp --> xrdp session
        * other types
        '''
        return 'xrdp' if 'XRDP_SESSION' in os.environ else os.environ.get('XDG_SESSION_TYPE', 'unknown')

    async def force_time_sync(self) -> None:
        return

    async def protect_file_for_owner_only(self, filepath: str) -> None:
        '''
        Protects a file so only owner can read/write
        '''
        try:
            os.chmod(filepath, 0o600)
        except Exception:
            pass

    async def set_title(self, title: str) -> None:
        setproctitle(title)

    # High level operations
    async def hlo_join_domain(self, name: str, custom: typing.Mapping[str, typing.Any]) -> bool:
        """Joins domain with given name and custom parameters"""
        await self.hlo_rename(name)

        await self.join_domain(custom=custom)
        return True
