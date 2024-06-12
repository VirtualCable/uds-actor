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
import collections.abc
import logging

from udsactor import types

logger = logging.getLogger(__name__)

if typing.TYPE_CHECKING:
    from udsactor.abc import ActorProcessor


class MessagesProcessor:
    actor: 'ActorProcessor'
    # Queue where messages from client or broker are received
    queue: asyncio.Queue[types.UDSMessage]
    # Queue where messages to be sent to client are put
    user_queue: asyncio.Queue[types.UDSMessage]

    logged_in: bool
    
    _processors:dict[
            types.UDSMessageType,
            typing.Callable[[types.UDSMessage], collections.abc.Awaitable[None]],
        ]

    def __init__(self, actor: 'ActorProcessor') -> None:
        self.actor = actor
        self.queue = asyncio.Queue()
        self.user_queue = asyncio.Queue()

        self.logged_in = False

        self._processors = {
            types.UDSMessageType.LOGIN: self.login,  # Client login
            types.UDSMessageType.LOGOUT: self.logout,  # This can be from client or broker
            types.UDSMessageType.CLOSE: self.logout,  # This can be from client only
            # PING and PONG are only used for keepalive, so we ignore them here
            # they are processed on ws server
            types.UDSMessageType.LOG: self.log,  # Log message, only from client
            # Messages from Broker
            types.UDSMessageType.SCRIPT: self.script,  # Script message, only from broker
            types.UDSMessageType.PRECONNECT: self.preconnect,  # Preconnect message from broker
        }

    async def login(self, msg: types.UDSMessage) -> None:
        self.logged_in = True
        await self.user_queue.put(
            types.UDSMessage(
                msg_type=types.UDSMessageType.LOGIN,
                data=(
                    await self.actor.login(username=msg.data['username'], session_type=msg.data['session_type'])
                ).as_dict(),
            )
        )

    async def logout(self, msg: types.UDSMessage) -> None:
        # If a request from the broker is received, send the logout request to the client queue
        req = types.LogoutRequest.from_dict(msg.data)
        if req.from_broker is True:
            await self.user_queue.put(msg)
            return
        if self.logged_in is False:
            return

        self.logged_in = False
        # Invoke actor logout
        await self.actor.logout(username=req.username, session_type=req.session_type, session_id=req.session_id)

    async def log(self, msg: types.UDSMessage) -> None:
        req = types.LogRequest.from_dict(msg.data)
        await self.actor.log(level=req.level, message=req.message)

    async def script(self, msg: types.UDSMessage) -> None:
        scrpt = types.ScriptRequest.from_dict(msg.data)
        if scrpt.as_user is True:
            await self.user_queue.put(msg)  # Send to client
        else:
            await self.actor.script(script=scrpt.script)

    async def preconnect(self, msg: types.UDSMessage) -> None:
        data = types.PreconnectRequest.from_dict(msg.data)
        await self.actor.preconnect(data=data)

    async def process_message(self, msg: types.UDSMessage) -> None:

        if msg.msg_type not in self._processors:
            logger.error('Unknown message type %s', msg.msg_type)
            return

        try:
            await self._processors[msg.msg_type](msg)
        except Exception:
            logger.exception('Exception processing message %s', msg.msg_type)

    async def run(self) -> None:
        while True:
            msg = await self.queue.get()
            await self.process_message(msg)
            self.queue.task_done()  # Allow join to work
