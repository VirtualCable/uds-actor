import random
import typing
import collections.abc
import string

from udsactor import types, consts


def configuration(
    token: str | None = None,
    udsserver_port: int = 0,
    actorType: types.ActorType = types.ActorType.MANAGED,
    **kwargs,
) -> types.ActorConfiguration:
    token = token or ''.join(random.choices(string.ascii_letters, k=32))  # nosec, test only
    return types.ActorConfiguration(
        version=consts.CONFIG_VERSION,
        actorType=types.ActorType.MANAGED,
        host=f'localhost:{udsserver_port}',        
        validateCertificate=False,
        token=token,
        log_level=types.LogLevel.DEBUG,
        **kwargs,
    )
