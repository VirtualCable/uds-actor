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
import random
import string
from typing import Any, Coroutine

import aiohttp

from udsactor import (
    server_msg_processor,
    types,
    consts,
    webserver,
    platform,
    server as server_module,
    managed,
    unmanaged,
)

from . import fixtures
from .cert import generate_cert


class NoReaderWriter(platform.abc.ConfigReader):
    cfg: types.ActorConfiguration

    def __init__(self, cfg: types.ActorConfiguration):
        self.cfg = cfg

    async def read(self) -> types.ActorConfiguration:
        return self.cfg

    async def write(self, cfg: types.ActorConfiguration) -> None:
        self.cfg = cfg

    async def scriptToInvokeOnLogin(self) -> str:
        return ''


@contextlib.asynccontextmanager
async def setup(
    udsserver_port: int = 8443, token: str | None = None, for_unmanaged: bool = False
) -> collections.abc.AsyncGenerator[aiohttp.ClientSession, None]:
    cfg = fixtures.configuration(token, udsserver_port)

    # Override "save" config, platform provided config, etc..
    p = platform.Platform.platform()
    # Ensure own is used
    p.cfgManager = NoReaderWriter(cfg)
    p.cfg = None  # Cleans up cfg

    # Now override
    server = server_module.UDSActorServer()
    actor = managed.ManagedActorProcessor() if not for_unmanaged else unmanaged.UnmanagedActorProcessor()
    msgServer = server_msg_processor.MessagesProcessor(actor=actor)

    notifier = asyncio.Event()
    # await webserver.server(cfg, generate_cert('127.0.0.1'), notifier)
    web_task = asyncio.create_task(
        webserver.server(
            cfg=cfg, certInfo=generate_cert('127.0.0.1'), serverMsgProcessor=msgServer, readyEvent=notifier
        )
    )

    msg_task = asyncio.create_task(msgServer.run())

    # Wait for server to be ready or task exception (not finish, because it will never finish until we cancel it)
    # create a future from notifier
    fut = asyncio.ensure_future(notifier.wait())
    # wait for notifier or task to finish
    await asyncio.wait([fut, web_task, msg_task], return_when=asyncio.FIRST_COMPLETED)
    # if any task exception, raise it
    if web_task.done() and not fut.done():
        web_task.result()
    if msg_task.done() and not fut.done():
        msg_task.result()

    # Create aiohttp client
    client = aiohttp.ClientSession(headers={'Content-Type': 'application/json'})
    try:
        yield client
    finally:
        try:
            web_task.cancel()
            msg_task.cancel()
            await asyncio.wait([web_task, msg_task], return_when=asyncio.ALL_COMPLETED)
        except asyncio.CancelledError:
            pass
        await client.close()
