#!/usr/bin/env python3
# -*- coding: utf-8 -*-
#
# Copyright (c) 2024 Virtual Cable S.L.U.
# All rights reserved.
#
# Redistribution and use in source and binary forms, with or without modification,
# are permitted provided that the following conditions are met:
#
#    * Redistributions of source code must retain the above copyright notice,
#      this list of conditions and the following disclaimer.
#    * Redistributions in binary form must reproduce the above copyright notice,
#      this list of conditions and the following disclaimer in the documentation
#      and/or other materials provided with the distribution.
#    * Neither the name of Virtual Cable S.L.U. nor the names of its contributors
#      may be used to endorse or promote products derived from this software
#      without specific prior written permission.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
# AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
# IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
# DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
# FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
# DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
# SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
# CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
# OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
# OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
'''
Author: Adolfo Gómez, dkmaster at dkmon dot com
'''
import asyncio
import typing
import logging

from udsactor import types, managed, unmanaged, rest

logger = logging.getLogger(__name__)

if typing.TYPE_CHECKING:
    from udsactor.abc import ActorProcessor


class MessagesProcessor:
    actor: 'ActorProcessor'
    incomingQueue: asyncio.Queue[types.UDSMessage]
    outgoingQueue: asyncio.Queue[types.UDSMessage]

    logged_in: bool

    def __init__(self, actor: 'ActorProcessor') -> None:
        self.actor = actor
        self.incomingQueue = asyncio.Queue()
        self.outgoingQueue = asyncio.Queue()

        self.logged_in = False

    async def process(self, proc: typing.Awaitable[typing.Any], msg: types.UDSMessage) -> None:
        try:
            result = await proc
        except Exception as e:
            if msg.callback:
                await msg.callback(None, e)
        else:
            if msg.callback:
                await msg.callback(result, None)

    async def login(self, msg: types.UDSMessage) -> None:
        self.logged_in = True
        await self.process(self.actor.login(msg.data['username'], msg.data['session_type']), msg)

    async def logout(self, msg: types.UDSMessage) -> None:
        if not self.logged_in:
            return

        self.logged_in = False
        await self.process(
            self.actor.logout(msg.data['username'], msg.data['session_type'], msg.data['session_id']), msg
        )

    async def log(self, msg: types.UDSMessage) -> None:
        await self.process(self.actor.log(msg.data['level'], msg.data['message']), msg)

    async def processMessage(self, msg: types.UDSMessage) -> None:
        processors: dict[types.UDSMessageType, typing.Callable[[types.UDSMessage], typing.Awaitable[None]]] = {
            types.UDSMessageType.LOGIN: self.login,
            types.UDSMessageType.LOGOUT: self.logout,
            types.UDSMessageType.CLOSE: self.logout,
            # PING and PONG are only used for keepalive, so we ignore them here
            # they are processed on ws server
            types.UDSMessageType.LOG: self.log,
        }

        if msg.msg_type not in processors:
            logger.error('Unknown message type %s', msg.msg_type)
            return

        await processors[msg.msg_type](msg)

    async def run(self) -> None:
        while True:
            msg = await self.incomingQueue.get()
            await self.processMessage(msg)
            self.incomingQueue.task_done()  # Allow join to work
