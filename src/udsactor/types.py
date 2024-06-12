#
# Copyright (c) 2023 Virtual Cable S.L.U.
# All rights reserved.
'''
Author: Adolfo Gómez, dkmaster at dkmon dot com
'''
import collections.abc
import datetime
import enum
import dataclasses
import typing


class LogLevel(enum.IntEnum):
    # From (logging + 10) * 1000    (logging.DEBUG = 10, logging.INFO = 20, etc..)
    OTHER = 10000
    DEBUG = 20000
    INFO = 30000
    WARNING = 40000
    ERROR = 50000
    CRITICAL = 60000

    def as_python(self) -> int:
        return (self.value // 1000) - 10

    @staticmethod
    def from_str(level: str) -> 'LogLevel':
        try:
            return LogLevel[level.upper()]
        except Exception:
            pass  # Ignore
        return LogLevel.ERROR  # If not found, return ERROR

    @staticmethod
    def from_python(level: int) -> 'LogLevel':
        try:
            return LogLevel(level * 1000 + 10000)
        except Exception:
            pass
        return LogLevel.ERROR  # Default to error


@dataclasses.dataclass(frozen=True)
class LogMessage:
    level: LogLevel
    message: str


class UDSMessageType(enum.StrEnum):
    MESSAGE = 'message'  # Data is the message to be shown (str)
    SCREENSHOT = 'screenshot'  # Data is the screenshot (bytes, png or jpeg) or None depending if it is for the server of for the client
    PRECONNECT = 'preconnect'  # Data is a PreconnectRequest
    SCRIPT = 'script'  # Data is a dict of {'script': str, 'args': typing.Optional[list[str]]} (Currently, for compat, args will be empty)
    LOGIN = 'login'  # Data is either a dict of {'username': str, 'session_type': str} or a LoginResultInfo
    LOGOUT = 'logout'  # Data is either a dict of {'username': str, 'session_type': str, 'session_id': str} or an Ok Message
    CLOSE = 'close'  # No data
    PING = 'ping'  # No data
    PONG = 'pong'  # No data
    LOG = 'log'  # Data is a dict of {'level': int, 'message': str}
    OK = 'ok'  # No data


@dataclasses.dataclass(frozen=True)
class UDSMessage:
    msg_type: UDSMessageType
    data: dict[str, typing.Any] = dataclasses.field(default_factory=dict)
    # Async callback, if any, to be called when message is processed
    callback: typing.Optional[
        typing.Callable[[typing.Any, typing.Optional[Exception]], typing.Coroutine[None, None, None]]
    ] = None

    def as_dict(self) -> dict[str, typing.Any]:
        # callback is not serialized
        return {
            'msg_type': self.msg_type.value,
            'data': self.data,
        }


@dataclasses.dataclass(frozen=True)
class InterfaceInfo:
    name: str
    mac: str
    ip: str  # IPv4 or IPv6


@dataclasses.dataclass(frozen=True)
class Authenticator:
    authId: str
    authSmallName: str
    auth: str
    type: str
    priority: int
    isCustom: bool


@dataclasses.dataclass(frozen=True)
class ActorOsConfiguration:
    action: str
    name: str
    custom: typing.Optional[collections.abc.Mapping[str, typing.Any]]

    @staticmethod
    def from_dict(
        data: typing.Optional[collections.abc.Mapping[str, typing.Any]] = None
    ) -> 'ActorOsConfiguration':
        if not data:
            return ActorOsConfiguration(action='', name='', custom=None)

        return ActorOsConfiguration(
            action=data.get('action', ''),
            name=data.get('name', ''),
            custom=data.get('custom', None),
        )

    def as_dict(self) -> dict[str, typing.Any]:
        return {
            'action': self.action,
            'name': self.name,
            'custom': self.custom,
        }


@dataclasses.dataclass(frozen=True)
class ActorDataConfiguration(typing.NamedTuple):
    unique_id: typing.Optional[str] = None
    os: typing.Optional[ActorOsConfiguration] = None

    @staticmethod
    def from_dict(
        data: typing.Optional[collections.abc.Mapping[str, typing.Any]] = None
    ) -> 'ActorDataConfiguration':
        if not data:
            return ActorDataConfiguration(unique_id=None, os=None)

        return ActorDataConfiguration(
            unique_id=data.get('unique_id', None),
            os=ActorOsConfiguration.from_dict(data.get('os', None)),
        )

    def as_dict(self) -> dict[str, typing.Any]:
        return {
            'unique_id': self.unique_id,
            'os': self.os.as_dict() if self.os else None,
        }


@dataclasses.dataclass
class ActorConfiguration:
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

    def as_dict(self) -> dict[str, typing.Any]:
        return {
            'version': self.version,
            'actorType': self.actorType,
            'token': self.token,
            'initialized': self.initialized,
            'host': self.host,
            'validateCertificate': self.validateCertificate,
            'restrict_net': self.restrict_net,
            'pre_command': self.pre_command,
            'runonce_command': self.runonce_command,
            'post_command': self.post_command,
            'log_level': self.log_level,
            'config': self.config.as_dict() if self.config else None,
            'data': self.data,
        }

    @staticmethod
    def from_dict(data: dict[str, typing.Any]) -> 'ActorConfiguration':
        if not data:
            raise Exception('Invalid data')
        cfg = data.copy()
        cfg['config'] = ActorDataConfiguration.from_dict(cfg.get('config', None))
        return ActorConfiguration(**cfg)


@dataclasses.dataclass(frozen=True)
class InitializationResult:
    token: typing.Optional[str] = None
    unique_id: typing.Optional[str] = None
    os: typing.Optional[ActorOsConfiguration] = None


@dataclasses.dataclass(frozen=True)
class LoginRequest:
    # {'username': '1234', 'session_type': 'test'}
    username: str
    session_type: str

    @staticmethod
    def null() -> 'LoginRequest':
        return LoginRequest(username='', session_type='')

    @staticmethod
    def from_dict(data: typing.Optional[collections.abc.Mapping[str, typing.Any]] = None) -> 'LoginRequest':
        if not data:
            return LoginRequest.null()

        return LoginRequest(
            username=data.get('username', ''),
            session_type=data.get('session_type', ''),
        )

    def as_dict(self) -> dict[str, typing.Any]:
        return {
            'username': self.username,
            'session_type': self.session_type,
        }


@dataclasses.dataclass(frozen=True)
class LoginResponse:
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
    def from_dict(data: typing.Optional[collections.abc.Mapping[str, typing.Any]] = None) -> 'LoginResponse':
        if not data:
            return LoginResponse.null()

        return LoginResponse(
            ip=data.get('ip', ''),
            hostname=data.get('hostname', ''),
            dead_line=data.get('dead_line', None),
            max_idle=data.get('max_idle', None),
            session_id=data.get('session_id', None),
        )

    def as_dict(self) -> dict[str, typing.Any]:
        return {
            'ip': self.ip,
            'hostname': self.hostname,
            'dead_line': self.dead_line,
            'max_idle': self.max_idle,
            'session_id': self.session_id,
        }


@dataclasses.dataclass(frozen=True)
class LogoutRequest:
    # {'username': '1234', 'session_type': 'test', 'session_id': 'test'}
    username: str
    session_id: str
    session_type: str = ''
    from_broker: bool = False

    @staticmethod
    def null(from_broker: bool = False) -> 'LogoutRequest':
        return LogoutRequest(username='', session_type='', session_id='', from_broker=from_broker)

    @staticmethod
    def from_dict(
        data: typing.Optional[collections.abc.Mapping[str, typing.Any]] = None, from_broker: bool = False
    ) -> 'LogoutRequest':
        if not data:
            return LogoutRequest.null()

        return LogoutRequest(
            username=data.get('username', ''),
            session_type=data.get('session_type', ''),
            session_id=data.get('session_id', ''),
            from_broker=from_broker or data.get('from_broker', False),
        )

    def as_dict(self) -> dict[str, typing.Any]:
        return {
            'username': self.username,
            'session_type': self.session_type,
            'session_id': self.session_id,
            'from_broker': self.from_broker,
        }


# No logout response class exists, just an OK message is sent


@dataclasses.dataclass(frozen=True)
class LogRequest:
    # {'level': 'INFO', 'message': 'test'}
    level: LogLevel
    message: str

    @staticmethod
    def null() -> 'LogRequest':
        return LogRequest(level=LogLevel.OTHER, message='')

    @staticmethod
    def from_dict(data: typing.Optional[collections.abc.Mapping[str, typing.Any]] = None) -> 'LogRequest':
        if not data:
            return LogRequest.null()

        return LogRequest(
            level=LogLevel.from_str(data.get('level', LogLevel.OTHER.name)),
            message=data.get('message', ''),
        )

    def as_dict(self) -> dict[str, typing.Any]:
        return {
            'level': self.level.name,
            'message': self.message,
        }


@dataclasses.dataclass(frozen=True)
class ClientInfo:
    url: str
    session_id: str


# Certificate
@dataclasses.dataclass(frozen=True)
class CertificateInfo:
    """A certificate"""

    key: str  # Key, in PEM format
    certificate: str  # Certificate, in PEM format
    password: str  # Password
    ciphers: typing.Optional[str] = None  # Ciphers to use (if None, default will be used)

    @staticmethod
    def from_dict(data: typing.Optional[collections.abc.Mapping[str, typing.Any]] = None) -> 'CertificateInfo':
        if not data:
            return CertificateInfo(key='', certificate='', password='', ciphers=None)

        return CertificateInfo(
            key=data.get('key', data.get('private_key', '')),
            certificate=data.get('certificate', data.get('server_certificate', '')),
            password=data.get('password', ''),
            ciphers=data.get('ciphers', None),
        )

    def as_dict(self) -> dict[str, typing.Any]:
        return {
            'key': self.key,
            'certificate': self.certificate,
            'password': self.password,
            'ciphers': self.ciphers,
        }


@dataclasses.dataclass(frozen=True)
class PreconnectRequest:
    #     self._params['user'],
    # self._params['protocol'],
    # self._params.get('ip', 'unknown'),
    # self._params.get('hostname', 'unknown'),
    # self._params.get('udsuser', 'unknown'),
    username: str  # From "user" or "username" param
    protocol: str
    ip: str
    hostname: str
    udsuser: str

    @staticmethod
    def null() -> 'PreconnectRequest':
        return PreconnectRequest(username='', protocol='', ip='', hostname='', udsuser='')

    @staticmethod
    def from_dict(
        data: typing.Optional[collections.abc.Mapping[str, typing.Any]] = None
    ) -> 'PreconnectRequest':
        if not data:
            return PreconnectRequest.null()

        return PreconnectRequest(
            username=data.get('username', data.get('user', '')),
            protocol=data.get('protocol', ''),
            ip=data.get('ip', ''),
            hostname=data.get('hostname', ''),
            udsuser=data.get('udsuser', ''),
        )

    def as_dict(self, compat: bool = False) -> dict[str, typing.Any]:
        data = {
            'protocol': self.protocol,
            'ip': self.ip,
            'hostname': self.hostname,
            'udsuser': self.udsuser,
        }
        if not compat:
            data['username'] = self.username
        else:
            data['user'] = self.username
        return data


@dataclasses.dataclass(frozen=True)
class ScriptRequest:
    # {'script': '# python code to execute'}
    script: str  # Script to execute
    as_user: typing.Optional[bool] = False  # If true, script will be executed as user, if false, as service
    script_type: str = 'python'  # Script type (python, bash, etc..)

    @staticmethod
    def null() -> 'ScriptRequest':
        return ScriptRequest(script='')

    @staticmethod
    def from_dict(data: typing.Optional[collections.abc.Mapping[str, typing.Any]] = None) -> 'ScriptRequest':
        if not data:
            return ScriptRequest.null()

        return ScriptRequest(
            script=data['script'],
            as_user=data.get('as_user', False),
            script_type=data.get('script_type', 'python'),
        )

    def as_dict(self) -> dict[str, typing.Any]:
        return {
            'script': self.script,
            'script_type': self.script_type,
            'as_user': self.as_user,
        }


# Cache related
@dataclasses.dataclass(frozen=True)
class CacheInfo:
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
