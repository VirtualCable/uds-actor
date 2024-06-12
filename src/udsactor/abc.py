# Copyright (c) 2023 Adolfo Gómez García <dkmaster@dkmon.com>
#
# This software is released under the MIT License.
# https://opensource.org/licenses/MIT

import typing
import abc
import logging

from udsactor import types, utils, rest
from udsactor.native import Manager

if typing.TYPE_CHECKING:
    from udsactor.server import UDSActorServer

logger = logging.getLogger(__name__)


class ActorProcessor(abc.ABC):
    platform: 'Manager'
    parent: 'UDSActorServer'

    _cfg: typing.Optional['types.ActorConfiguration']
    _api: typing.Optional['rest.BrokerREST']

    def __init__(self, parent: 'UDSActorServer') -> None:
        self.platform = Manager.instance()
        self.parent = parent
        self._cfg = None
        self._api = None

    @property
    async def config(self) -> types.ActorConfiguration:
        if not self._cfg:
            self._cfg = await self.platform.config
        return self._cfg

    @property
    async def api(self) -> 'rest.BrokerREST':
        cfg = await self.config
        if not self._api:
            self._api = rest.BrokerREST(cfg.host, cfg.validateCertificate, cfg.token)
        return self._api

    async def log(self, *, level: types.LogLevel, message: str) -> None:
        """
        Logs the requested message on this platform
        USed to redirect client logs to server
        """
        logger.log(level.as_python(), 'SESSION: %s', message)

    async def script(self, *, script: str) -> None:
        """
        Executes the script on this platform

        Normally, commmon code will be fine for this, so we provide a default implementation

        Args:
            script: Script to execute (python code currently only)

        Returns:
            None
        """
        await utils.script_executor(script)

    async def preconnect(self, *, data: types.PreconnectRequest) -> None:
        """
        Preconnects to the requested service

        Args:
            data: Preconnect request data

        Returns:
            None
        """
        await self.parent.preconnect(data=data)

    @abc.abstractmethod
    async def initialize(
        self, *, interfaces: list[types.InterfaceInfo]
    ) -> typing.Optional[types.CertificateInfo]:
        pass

    @abc.abstractmethod
    async def login(self, *, username: str, session_type: str) -> types.LoginResponse:
        pass

    @abc.abstractmethod
    async def logout(self, *, username: str, session_type: str, session_id: str) -> None:
        pass
