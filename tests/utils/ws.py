import asyncio
import typing
import collections.abc
import logging
import contextlib
from unittest import mock


from udsactor import exceptions, rest, managed, consts, types

from . import rest_server, fixtures, fake_uds_server, exclusive_tests

logger = logging.getLogger(__name__)


class WSConnectionResult(typing.NamedTuple):
    private: rest.PrivateREST
    broker_server: fake_uds_server.FakeUDSRestServer
    local_server: rest_server.SetupResult
    # List of exceptions
    excpts: list[Exception]
    task: asyncio.Task[None]


@contextlib.asynccontextmanager
async def ws_connection(
    processor: collections.abc.Callable[[types.UDSMessage], collections.abc.Awaitable[None]],
    timeout: float = 10,
) -> typing.AsyncIterator[WSConnectionResult]:
    excpts: list[Exception] = []

    priv = rest.PrivateREST(ipv6=False)
    async with fake_uds_server.fake_uds_rest_server() as server:
        async with rest_server.setup(udsserver_port=server.port, token=fake_uds_server.TOKEN) as local_server:

            async def inner_process(msg: types.UDSMessage) -> None:
                try:
                    await processor(msg)
                except exceptions.RequestStop:
                    raise
                except Exception as e:
                    excpts.append(e)
                    raise exceptions.RequestStop()

            async def timeout_task_runner() -> None:
                try:
                    await asyncio.wait_for(priv.communicate(inner_process), timeout)
                except asyncio.TimeoutError:
                    excpts.append(asyncio.TimeoutError())

            task = asyncio.create_task(timeout_task_runner())
            
            yield WSConnectionResult(private=priv, broker_server=server, local_server=local_server, excpts=excpts, task=task)
