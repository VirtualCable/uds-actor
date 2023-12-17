#
# Copyright (c) 2023 Virtual Cable S.L.U.
# All rights reserved.
'''
Author: Adolfo Gómez, dkmaster at dkmon dot com
'''
import collections.abc
import datetime
import enum
import functools
import hashlib
import typing

from . import consts


class LogLevel(enum.IntEnum):
    # From (logging + 10) * 1000    (logging.DEBUG = 10, logging.INFO = 20, etc..)
    OTHER = 10000
    DEBUG = 20000
    INFO = 30000
    WARNING = 40000
    ERROR = 50000
    CRITICAL = 60000

    def asPython(self) -> int:
        return (self.value // 1000) - 10

    @staticmethod
    def fromStr(level: str) -> 'LogLevel':
        try:
            return LogLevel[level.upper()]
        except Exception:
            pass  # Ignore
        return LogLevel.ERROR  # If not found, return ERROR

    @staticmethod
    def fromPython(level: int) -> 'LogLevel':
        try:
            return LogLevel(level * 1000 + 10000)
        except Exception:
            pass
        return LogLevel.ERROR  # Default to error


class UDSMessageType(enum.StrEnum):
    MESSAGE = 'message'
    SCREENSHOT = 'screenshot'
    LOGIN = 'login'
    LOGOUT = 'logout'
    CLOSE = 'close'
    PING = 'ping'
    PONG = 'pong'
    LOG = 'log'


class UDSMessage(typing.NamedTuple):
    msg_type: UDSMessageType
    data: typing.Any = None
    # Async callback, if any, to be called when message is processed
    callback: typing.Optional[
        typing.Callable[[typing.Any, typing.Optional[Exception]], typing.Coroutine]
    ] = None

    def asDict(self) -> dict[str, typing.Any]:
        # callback is not serialized
        return {
            'msg_type': self.msg_type.value,
            'data': self.data,
        }


class InterfaceInfo(typing.NamedTuple):
    name: str
    mac: str
    ip: str  # IPv4 or IPv6


class Authenticator(typing.NamedTuple):
    authId: str
    authSmallName: str
    auth: str
    type: str
    priority: int
    isCustom: bool


class ActorOsConfiguration(typing.NamedTuple):
    action: str
    name: str
    custom: typing.Optional[collections.abc.Mapping[str, typing.Any]]


class ActorDataConfiguration(typing.NamedTuple):
    unique_id: typing.Optional[str] = None
    os: typing.Optional[ActorOsConfiguration] = None


class ActorConfiguration(typing.NamedTuple):
    version: int = 0  # No version, old version
    actorType: typing.Optional[str] = None

    token: typing.Optional[str] = None
    initialized: typing.Optional[bool] = None

    host: str = ''
    validateCertificate: bool = True

    restrict_net: typing.Optional[str] = None

    pre_command: typing.Optional[str] = None
    runonce_command: typing.Optional[str] = None
    post_command: typing.Optional[str] = None

    log_level: int = 2

    config: typing.Optional[ActorDataConfiguration] = None

    data: typing.Optional[dict[str, typing.Any]] = None

    def is_empty(self) -> bool:
        return not bool(self.host) or not bool(self.token)

    def asDict(self) -> dict[str, typing.Any]:
        cfg = self._asdict()
        cfg['config'] = cfg['config']._asdict() if cfg['config'] else None
        return cfg

    @staticmethod
    def fromDict(data: dict[str, typing.Any]) -> 'ActorConfiguration':
        cfg = data.copy()
        cfg['config'] = ActorDataConfiguration(**cfg['config']) if cfg['config'] else None
        return ActorConfiguration(**cfg)


class InitializationResult(typing.NamedTuple):
    token: typing.Optional[str] = None
    unique_id: typing.Optional[str] = None
    os: typing.Optional[ActorOsConfiguration] = None


class LoginResultInfo(typing.NamedTuple):
    ip: str
    hostname: str
    dead_line: typing.Optional[int]
    max_idle: typing.Optional[int]
    session_id: typing.Optional[str]

    @property
    def is_logged_in(self) -> bool:
        return bool(self.session_id)
    
    @property
    def is_empty(self) -> bool:
        return not bool(self.ip) and not bool(self.hostname)

    @staticmethod
    def fromDict(data: typing.Optional[collections.abc.Mapping[str, typing.Any]] = None) -> 'LoginResultInfo':
        if not data:
            return LoginResultInfo(
                ip='0.0.0.0', hostname=consts.UNKNOWN, dead_line=None, max_idle=None, session_id=None
            )

        return LoginResultInfo(
            ip=data.get('ip', ''),
            hostname=data.get('hostname', ''),
            dead_line=data.get('dead_line', None),
            max_idle=data.get('max_idle', None),
            session_id=data.get('session_id', None),
        )

    def asDict(self) -> dict[str, typing.Any]:
        return self._asdict()
    


class ClientInfo(typing.NamedTuple):
    url: str
    session_id: str


# Certificate
class CertificateInfo(typing.NamedTuple):
    """A certificate"""

    key: str  # Key, in PEM format
    certificate: str  # Certificate, in PEM format
    password: str  # Password
    ciphers: typing.Optional[str] = None  # Ciphers to use (if None, default will be used)

    @staticmethod
    def fromDict(data: typing.Optional[collections.abc.Mapping[str, typing.Any]] = None) -> 'CertificateInfo':
        if not data:
            return CertificateInfo(key='', certificate='', password='', ciphers=None)

        return CertificateInfo(
            key=data.get('key', data.get('private_key', '')),
            certificate=data.get('certificate', data.get('server_certificate', '')),
            password=data.get('password', ''),
            ciphers=data.get('ciphers', None),
        )

    def asDict(self) -> dict[str, typing.Any]:
        return self._asdict()


# Cache related
class CacheInfo(typing.NamedTuple):
    """
    Cache info
    """

    hits: int
    misses: int
    maxsize: int
    currsize: int


# Cache duration is in fact a timedelta right now, but we might want to change it?
CacheDuration = datetime.timedelta


class ActorType(enum.StrEnum):
    MANAGED = 'managed'
    UNMANAGED = 'unmanaged'


class ApiType(enum.IntEnum):
    ACTORV3 = 0
    AUTH = 1
