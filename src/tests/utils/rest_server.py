#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""

import typing
import collections.abc
import asyncio
import contextlib

import aiohttp

from udsactor import (
    server_msg_processor,
    webserver,
    server as server_module,
    managed,
    unmanaged,
    abc,
)

from . import fixtures, tools
from .cert import generate_cert


class SetupResult(typing.NamedTuple):
    client: aiohttp.ClientSession
    actor: abc.ActorProcessor
    msg_processor: server_msg_processor.MessagesProcessor
    actor_server: server_module.UDSActorServer


@contextlib.asynccontextmanager
async def setup(
    udsserver_port: int = 8443, token: str | None = None, for_unmanaged: bool = False,
    do_not_start_msg_processor: bool = False
) -> collections.abc.AsyncGenerator[SetupResult, None]:
    cfg = fixtures.configuration(token, udsserver_port)

    tools.set_testing_cfg(cfg)

    # Now override
    server = server_module.UDSActorServer()
    actor = managed.ManagedActorProcessor(parent=server) if not for_unmanaged else unmanaged.UnmanagedActorProcessor(parent=server)
    msg_server = server_msg_processor.MessagesProcessor(actor=actor)

    notifier = asyncio.Event()
    # await webserver.server(cfg, generate_cert('127.0.0.1'), notifier)
    web_task = asyncio.create_task(
        webserver.server(
            cfg=cfg, cert_info=generate_cert('127.0.0.1'), server_msg_processor=msg_server, ready_event=notifier
        )
    )

    msg_task = asyncio.create_task(msg_server.run()) if not do_not_start_msg_processor else None

    # Wait for server to be ready or task exception (not finish, because it will never finish until we cancel it)
    # create a future from notifier
    fut = asyncio.ensure_future(notifier.wait())
    # wait for notifier or task to finish
    tasks = [fut, web_task] + ([msg_task] if msg_task else [])
    await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
    # if any task exception, raise it
    if not fut.done():
        if web_task.done():
            web_task.result()
        if msg_task and msg_task.done():
            msg_task.result()

    # Create aiohttp client
    client = aiohttp.ClientSession(headers={'Content-Type': 'application/json'})
    try:
        yield SetupResult(client=client, msg_processor=msg_server, actor_server=server, actor=actor)
    finally:
        try:
            web_task.cancel()
            if msg_task:
                msg_task.cancel()
            await asyncio.wait(tasks, return_when=asyncio.ALL_COMPLETED)
        except asyncio.CancelledError:
            pass
        await client.close()
