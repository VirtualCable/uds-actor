import base64
import pickle
import typing
import collections.abc

MANAGED = 'managed'
UNMANAGED = 'unmanaged'


class InterfaceInfoType(typing.NamedTuple):
    name: str
    mac: str
    ip: str


class AuthenticatorType(typing.NamedTuple):
    authId: str
    authSmallName: str
    auth: str
    type: str
    priority: int
    isCustom: bool


class ActorOsConfigurationType(typing.NamedTuple):
    action: str
    name: str
    custom: typing.Optional[collections.abc.Mapping[str, typing.Any]]

    def as_dict(self) -> dict[str, typing.Any]:
        return {
            "action": self.action,
            "name": self.name,
            "custom": self.custom,
        }

    @staticmethod
    def from_dict(data: dict[str, typing.Any]) -> 'ActorOsConfigurationType':
        return ActorOsConfigurationType(
            action=data.get("action", ""),
            name=data.get("name", ""),
            custom=data.get("custom"),
        )


class ActorDataConfigurationType(typing.NamedTuple):
    unique_id: typing.Optional[str] = None
    os: typing.Optional[ActorOsConfigurationType] = None

    def as_dict(self) -> dict[str, typing.Any]:
        return {
            "unique_id": self.unique_id,
            "os": self.os.as_dict() if self.os else None,
        }

    @staticmethod
    def from_dict(data: dict[str, typing.Any]) -> 'ActorDataConfigurationType':
        return ActorDataConfigurationType(
            unique_id=data.get("unique_id"),
            os=ActorOsConfigurationType.from_dict(data["os"]) if data.get("os") else None,
        )


class ActorConfigurationType(typing.NamedTuple):
    host: str
    check_certificate: bool
    actor_type: typing.Optional[str] = None
    master_token: typing.Optional[str] = None
    own_token: typing.Optional[str] = None
    restrict_net: typing.Optional[str] = None

    pre_command: typing.Optional[str] = None
    runonce_command: typing.Optional[str] = None
    post_command: typing.Optional[str] = None

    log_level: int = 2

    config: typing.Optional[ActorDataConfigurationType] = None

    data: typing.Optional[dict[str, typing.Any]] = None

    def as_dict(self) -> dict[str, typing.Any]:
        subdata = (
            base64.b64encode(pickle.dumps(self.data)).decode() if self.data else None
        )  # nosec: file is restricted
        return {
            "host": self.host,
            "check_certificate": self.check_certificate,
            "actor_type": self.actor_type,
            "master_token": self.master_token,
            "own_token": self.own_token,
            "restrict_net": self.restrict_net,
            "pre_command": self.pre_command,
            "runonce_command": self.runonce_command,
            "post_command": self.post_command,
            "log_level": self.log_level,
            "config": self.config,
            "data": subdata
        }

    @staticmethod
    def from_dict(data: dict[str, typing.Any]) -> 'ActorConfigurationType':
        base64_data = data.get('data', None)
        subdata = (
            pickle.loads(base64.b64decode(base64_data.encode()))  # nosec: file is restricted
            if base64_data
            else None
        )
        return ActorConfigurationType(
            host=data.get("host", ""),
            check_certificate=data.get("check_certificate", False),
            actor_type=data.get("actor_type"),
            master_token=data.get("master_token"),
            own_token=data.get("own_token"),
            restrict_net=data.get("restrict_net"),
            pre_command=data.get("pre_command"),
            runonce_command=data.get("runonce_command"),
            post_command=data.get("post_command"),
            log_level=data.get("log_level", 2),
            config=ActorDataConfigurationType.from_dict(data["config"]) if data.get("config") else None,
            data=subdata,
        )


class InitializationResultType(typing.NamedTuple):
    master_token: typing.Optional[str] = None
    token: typing.Optional[str] = None
    unique_id: typing.Optional[str] = None
    os: typing.Optional[ActorOsConfigurationType] = None


class LoginResultInfoType(typing.NamedTuple):
    ip: str
    hostname: str
    deadline: typing.Optional[int]
    max_idle: typing.Optional[int]
    session_id: typing.Optional[str]

    @property
    def logged_in(self) -> bool:
        return bool(self.session_id)


class ClientInfo(typing.NamedTuple):
    url: str
    session_id: str


class CertificateInfoType(typing.NamedTuple):
    private_key: str
    server_certificate: str
    password: str
    ciphers: str
