# Copyright (c) 2023 Adolfo Gómez García <dkmaster@dkmon.com>
#
# This software is released under the MIT License.
# https://opensource.org/licenses/MIT

import typing
import abc
import logging
import collections.abc

from udsactor import types, utils

logger = logging.getLogger(__name__)

class Operations(abc.ABC):
    @abc.abstractmethod
    async def is_user_admin(self) -> bool:
        pass

    @abc.abstractmethod
    async def computer_name(self) -> str:
        pass

    @abc.abstractmethod
    async def list_interfaces(self) -> list[types.InterfaceInfo]:
        pass

    @abc.abstractmethod
    async def domain_name(self) -> str:
        pass

    @abc.abstractmethod
    async def os_name(self) -> str:
        pass

    @abc.abstractmethod
    async def os_version(self) -> str:
        pass

    @abc.abstractmethod
    async def reboot(self, flags: int = 0) -> None:
        pass

    @abc.abstractmethod
    async def loggoff(self) -> None:
        pass

    @abc.abstractmethod
    async def rename_computer(self, newName: str) -> bool:
        pass

    @abc.abstractmethod
    async def join_domain(self, **kwargs: typing.Any) -> None:
        pass

    @abc.abstractmethod
    async def change_user_password(self, user: str, oldPassword: str, newPassword: str) -> None:
        pass

    @abc.abstractmethod
    async def init_idle_duration(self, atLeastSeconds: int) -> None:
        pass

    @abc.abstractmethod
    async def current_idle(self) -> float:
        pass

    @abc.abstractmethod
    async def whoami(self) -> str:
        pass

    @abc.abstractmethod
    async def session_type(self) -> str:
        pass

    @abc.abstractmethod
    async def force_time_sync(self) -> None:
        pass

    @abc.abstractmethod
    async def protect_file_for_owner_only(self, filepath: str) -> None:
        pass

    @abc.abstractmethod
    async def set_title(self, title: str) -> None:
        pass
    
    # High level operations
    @abc.abstractmethod
    async def hlo_join_domain(self, name: str, custom: collections.abc.Mapping[str, typing.Any]) -> bool:
        """Joins domain with given name and custom parameters
        
        Returns true if a reboot is needed, false if not
        """
        pass

    
    # Default implementations
    async def hlo_rename(
        self,
        name: str,
        username: typing.Optional[str] = None,
        old_password: typing.Optional[str] = None,
        new_password: typing.Optional[str] = None,
    ) -> bool:
        '''
        Rename computer and change password if needed
        '''
        hostName = await self.computer_name()

        # Check for password change request for an user
        if username and new_password:
            logger.info('Setting password for configured user')
            try:
                await self.change_user_password(username, old_password or '', new_password)
            except Exception as e:
                # Logs error, but continue renaming computer
                logger.error('Could not change password for user {}: {}'.format(username, e))

        if hostName.lower() == name.lower():
            logger.info('Computer name is already {}'.format(hostName))
            return False

        return await self.rename_computer(name)
    

    # Convenient, overridable methods
    async def list_valid_interfaces(self, net_filter: typing.Optional[str] = None) -> list['types.InterfaceInfo']:
        cards = await self.list_interfaces()
        try:
            subnet = utils.str_to_net(net_filter)
        except Exception:
            subnet = None

        if subnet is None:
            return list(cards)

        return [c for c in cards if utils.ip_in_net(c.ip, subnet)]


class ConfigReader(abc.ABC):
    @abc.abstractmethod
    async def read(self) -> types.ActorConfiguration:
        pass

    @abc.abstractmethod
    async def write(self, config: types.ActorConfiguration) -> None:
        pass

    @abc.abstractmethod
    async def script_to_invoke_on_login(self) -> str:
        pass


class Runner(abc.ABC):
    # Not async, as this is the main thread
    @abc.abstractmethod
    def run(self) -> None:
        pass
