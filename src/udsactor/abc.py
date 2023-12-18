# Copyright (c) 2023 Adolfo Gómez García <dkmaster@dkmon.com>
#
# This software is released under the MIT License.
# https://opensource.org/licenses/MIT

import typing
import abc
import logging
import collections.abc

from udsactor import types, utils, rest
from udsactor.native import Manager

if typing.TYPE_CHECKING:
    from udsactor.server import UDSActorServer

logger = logging.getLogger(__name__)


class ActorProcessor(abc.ABC):
    platform: 'Manager'
    _cfg: typing.Optional['types.ActorConfiguration']
    _api: typing.Optional['rest.BrokerREST']

    def __init__(self) -> None:
        self.platform = Manager.instance()
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

    async def log(self, level: types.LogLevel, msg: str) -> None:
        """
        Logs the requested message on this platform
        USed to redirect client logs to server
        """
        logger.log(level.asPython(), 'SESSION: %s', msg)

    @abc.abstractmethod
    async def initialize(self, interfaces: list[types.InterfaceInfo]) -> typing.Optional[types.CertificateInfo]:
        pass

    @abc.abstractmethod
    async def login(self, username: str, sessionType: str) -> types.LoginResponse:
        pass

    @abc.abstractmethod
    async def logout(self, username: str, session_type: str, session_id: str) -> None:
        pass
