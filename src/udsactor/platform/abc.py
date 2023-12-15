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
    async def IsUserAnAdmin(self) -> bool:
        pass

    @abc.abstractmethod
    async def getComputerName(self) -> str:
        pass

    @abc.abstractmethod
    async def getNetworkInfo(self) -> list[types.InterfaceInfo]:
        pass

    @abc.abstractmethod
    async def getDomainName(self) -> str:
        pass

    @abc.abstractmethod
    async def getOSName(self) -> str:
        pass

    @abc.abstractmethod
    async def getOSVersion(self) -> str:
        pass

    @abc.abstractmethod
    async def reboot(self, flags: int = 0) -> None:
        pass

    @abc.abstractmethod
    async def loggoff(self) -> None:
        pass

    @abc.abstractmethod
    async def renameComputer(self, newName: str) -> bool:
        pass

    @abc.abstractmethod
    async def joinDomain(self, **kwargs: typing.Any) -> None:
        pass

    @abc.abstractmethod
    async def changeUserPassword(self, user: str, oldPassword: str, newPassword: str) -> None:
        pass

    @abc.abstractmethod
    async def initIdleDuration(self, atLeastSeconds: int) -> None:
        pass

    @abc.abstractmethod
    async def getIdleDuration(self) -> float:
        pass

    @abc.abstractmethod
    async def getCurrentUser(self) -> str:
        pass

    @abc.abstractmethod
    async def getSessionType(self) -> str:
        pass

    @abc.abstractmethod
    async def forceTimeSync(self) -> None:
        pass

    @abc.abstractmethod
    async def protectFileForOwnerOnly(self, filepath: str) -> None:
        pass

    @abc.abstractmethod
    async def setTitle(self, title: str) -> None:
        pass
    
    # High level operations
    @abc.abstractmethod
    async def hloJoinDomain(self, name: str, custom: collections.abc.Mapping[str, typing.Any]) -> bool:
        """Joins domain with given name and custom parameters
        
        Returns true if a reboot is needed, false if not
        """
        pass

    
    # Default implementations
    async def hloRename(
        self,
        name: str,
        userName: typing.Optional[str] = None,
        oldPassword: typing.Optional[str] = None,
        newPassword: typing.Optional[str] = None,
    ) -> bool:
        '''
        Rename computer and change password if needed
        '''
        hostName = await self.getComputerName()

        # Check for password change request for an user
        if userName and newPassword:
            logger.info('Setting password for configured user')
            try:
                await self.changeUserPassword(userName, oldPassword or '', newPassword)
            except Exception as e:
                # Logs error, but continue renaming computer
                logger.error('Could not change password for user {}: {}'.format(userName, e))

        if hostName.lower() == name.lower():
            logger.info('Computer name is already {}'.format(hostName))
            return False

        return await self.renameComputer(name)
    

    # Convenient, overridable methods
    async def validNetworkCards(self, netString: typing.Optional[str] = None) -> list['types.InterfaceInfo']:
        cards = await self.getNetworkInfo()
        try:
            subnet = utils.str_to_net(netString)
        except Exception as e:
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
    async def scriptToInvokeOnLogin(self) -> str:
        pass


class Runner(abc.ABC):
    # Not async, as this is the main thread
    @abc.abstractmethod
    def run(self) -> None:
        pass
