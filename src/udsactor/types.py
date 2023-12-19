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
    MESSAGE = 'message'  # Data is the message to be shown (str), response is an Ok Message
    SCREENSHOT = 'screenshot'  # Data is the screenshot (bytes, png or jpeg) or None depending if it is for the server of for the client
    LOGIN = 'login'  # Data is either a dict of {'username': str, 'session_type': str} or a LoginResultInfo, depending if it is for the server of for the client
    LOGOUT = 'logout'  # Data is either a dict of {'username': str, 'session_type': str, 'session_id': str} or an Ok Message, depending if it is for the server of for the client
    CLOSE = 'close'  # No data, response is an Ok Message
    PING = 'ping'  # No data, response is a Pong Message
    PONG = 'pong'  # No data, only used in response to a ping
    LOG = 'log'  # Data is a dict of {'level': int, 'message': str}, response is an Ok Message
    OK = 'ok'  # No data, This is a response to a message that does not need a response :)


class UDSMessage(typing.NamedTuple):
    msg_type: UDSMessageType
    data: dict[str, typing.Any] = {}
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

    @property
    def is_null(self) -> bool:
        return not bool(self.host) or not bool(self.token)

    def asDict(self) -> dict[str, typing.Any]:
        cfg = self._asdict()
        cfg['config'] = cfg['config']._asdict() if cfg['config'] else None
        return cfg

    @staticmethod
    def fromDict(data: dict[str, typing.Any]) -> 'ActorConfiguration':
        if not data or not isinstance(data, collections.abc.Mapping):
            raise Exception('Invalid data')

        cfg = data.copy()
        cfg['config'] = ActorDataConfiguration(**cfg['config']) if cfg['config'] else None
        return ActorConfiguration(**cfg)


class InitializationResult(typing.NamedTuple):
    token: typing.Optional[str] = None
    unique_id: typing.Optional[str] = None
    os: typing.Optional[ActorOsConfiguration] = None


class LoginRequest(typing.NamedTuple):
    # {'username': '1234', 'session_type': 'test'}
    username: str
    session_type: str

    @staticmethod
    def null() -> 'LoginRequest':
        return LoginRequest(username='', session_type='')

    @staticmethod
    def fromDict(data: typing.Optional[collections.abc.Mapping[str, typing.Any]] = None) -> 'LoginRequest':
        if not data or not isinstance(data, collections.abc.Mapping):
            return LoginRequest.null()

        return LoginRequest(
            username=data.get('username', ''),
            session_type=data.get('session_type', ''),
        )

    def asDict(self) -> dict[str, typing.Any]:
        return self._asdict()


class LoginResponse(typing.NamedTuple):
    ip: str
    hostname: str
    dead_line: typing.Optional[int]
    max_idle: typing.Optional[int]
    session_id: typing.Optional[str]

    @property
    def is_logged_in(self) -> bool:
        return bool(self.session_id)

    @property
    def is_null(self) -> bool:
        return not bool(self.ip) and not bool(self.hostname)

    @staticmethod
    def null() -> 'LoginResponse':
        return LoginResponse(ip='', hostname='', dead_line=None, max_idle=None, session_id=None)

    @staticmethod
    def fromDict(data: typing.Optional[collections.abc.Mapping[str, typing.Any]] = None) -> 'LoginResponse':
        if not data or not isinstance(data, collections.abc.Mapping):
            return LoginResponse.null()

        return LoginResponse(
            ip=data.get('ip', ''),
            hostname=data.get('hostname', ''),
            dead_line=data.get('dead_line', None),
            max_idle=data.get('max_idle', None),
            session_id=data.get('session_id', None),
        )

    def asDict(self) -> dict[str, typing.Any]:
        return self._asdict()


class LogoutRequest(typing.NamedTuple):
    # {'username': '1234', 'session_type': 'test', 'session_id': 'test'}
    username: str
    session_id: str
    session_type: str = ''

    @staticmethod
    def null() -> 'LogoutRequest':
        return LogoutRequest(username='', session_type='', session_id='')

    @staticmethod
    def fromDict(data: typing.Optional[collections.abc.Mapping[str, typing.Any]] = None) -> 'LogoutRequest':
        if not data or not isinstance(data, collections.abc.Mapping):
            return LogoutRequest.null()

        return LogoutRequest(
            username=data.get('username', ''),
            session_type=data.get('session_type', ''),
            session_id=data.get('session_id', ''),
        )

    def asDict(self) -> dict[str, typing.Any]:
        return self._asdict()


# No logout response, just an OK message


class LogRequest(typing.NamedTuple):
    # {'level': 'INFO', 'message': 'test'}
    level: LogLevel
    message: str

    @staticmethod
    def null() -> 'LogRequest':
        return LogRequest(level=LogLevel.OTHER, message='')

    @staticmethod
    def fromDict(data: typing.Optional[collections.abc.Mapping[str, typing.Any]] = None) -> 'LogRequest':
        if not data or not isinstance(data, collections.abc.Mapping):
            return LogRequest.null()

        return LogRequest(
            level=LogLevel.fromStr(data.get('level', LogLevel.OTHER.name)),
            message=data.get('message', ''),
        )

    def asDict(self) -> dict[str, typing.Any]:
        return {
            'level': self.level.name,
            'message': self.message,
        }


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
        if not data or not isinstance(data, collections.abc.Mapping):
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
