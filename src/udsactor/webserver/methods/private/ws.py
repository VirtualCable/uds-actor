#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import asyncio
import logging
import typing

import aiohttp
import aiohttp.web

from udsactor import consts, types

from ...routes import routes
from ...keys import MSGS_PROCESSOR_KEY

logger = logging.getLogger(__name__)

if typing.TYPE_CHECKING:
    # from udsactor import rest, server_msg_processor
    pass


@routes.get(consts.PRIVATE_REST_WS)
async def ws(request: aiohttp.web.Request) -> aiohttp.web.WebSocketResponse:
    """Processes a launch request (from local)

    The client expects a json response from types.LoginResult
    """
    # udsRest = typing.cast('rest.BrokerREST', request.app[UDSREST_KEY])
    # Our outgoing queue is the incoming queue of the processor
    outgoing_queue = request.app[MSGS_PROCESSOR_KEY].queue

    # Our incoming queue is the outgoing queue of the processor
    incoming_queue = request.app[MSGS_PROCESSOR_KEY].user_queue

    # On connection, ensure all messages are cleared
    while not incoming_queue.empty():
        incoming_queue.get_nowait()

    ws = aiohttp.web.WebSocketResponse()
    await ws.prepare(request)
    logger.debug('Websocket connection ready')

    # helper to send Ok message
    async def send_ok() -> None:  # pyright: ignore[reportUnusedFunction]
        await ws.send_json(types.UDSMessage(msg_type=types.UDSMessageType.OK).as_dict())

    # Process incomming messages and also process messages from queue
    async def process_queue() -> None:
        while True:
            msg = await incoming_queue.get()
            await ws.send_json(msg.as_dict())

    async def process_ws() -> None:
        try:
            async for msg in ws:
                if msg.type == aiohttp.WSMsgType.TEXT:
                    data = msg.json()
                    message = types.UDSMessage(**data)
                    if message.msg_type == types.UDSMessageType.CLOSE:
                        # Close connection, respond to it an notify logout to outgoing queue
                        await outgoing_queue.put(
                            types.UDSMessage(msg_type=types.UDSMessageType.LOGOUT, data={})
                        )
                        await ws.close()
                    elif message.msg_type == types.UDSMessageType.PING:
                        # Put a pong message in the queue
                        await ws.send_json(types.UDSMessage(msg_type=types.UDSMessageType.PONG).as_dict())
                    elif message.msg_type == types.UDSMessageType.LOG:
                        await outgoing_queue.put(message)
                    elif message.msg_type == types.UDSMessageType.LOGIN:
                        await outgoing_queue.put(message)
                    elif message.msg_type == types.UDSMessageType.LOGOUT:
                        await outgoing_queue.put(message)
                    elif message.msg_type == types.UDSMessageType.SCREENSHOT:
                        await outgoing_queue.put(message)
                    else:
                        # Log strange messages
                        logger.warning('Unknown message received: %s', message)
                elif msg.type == aiohttp.WSMsgType.ERROR:
                    logger.error('ws connection closed with exception %s', ws.exception())
        except asyncio.CancelledError:
            logger.debug('Websocket connection cancelled')
        except Exception as e:
            logger.exception('Exception on websocket connection: %s', e)

    # Start tasks, and wait for them to finish (first one to finish will close the connection)
    await asyncio.gather(process_queue(), process_ws())
    logger.debug('Websocket connection closed')
    return ws