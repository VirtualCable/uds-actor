import typing

from udsactor import types, consts

from . import tools


def configuration(
    token: str | None = None,
    udsserver_port: int = 0,
    actorType: types.ActorType = types.ActorType.MANAGED,
    **kwargs: typing.Any,
) -> types.ActorConfiguration:
    token = token or tools.rnd_string_for_test(length=32)
    return types.ActorConfiguration(
        version=consts.CONFIG_VERSION,
        actorType=actorType,
        host=f'localhost:{udsserver_port}',
        validateCertificate=False,
        token=token,
        log_level=types.LogLevel.DEBUG,
        **kwargs,
    )
