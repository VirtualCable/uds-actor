#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import logging
import asyncio
import typing

import aiohttp
import aiohttp.web

from udsactor import consts, types

from ...routes import routes
from ...keys import MSGS_QUEUE_KEY

logger = logging.getLogger(__name__)

if typing.TYPE_CHECKING:
    from udsactor import rest, server_msg_processor


@routes.post(consts.PRIVATE_REST_LOGIN)
async def login(request: aiohttp.web.Request) -> aiohttp.web.Response:
    """Processes a launch request (from local)

    The client expects a json response from types.LoginResult
    """
    outgoingQueue: asyncio.Queue = typing.cast(
        'server_msg_processor.MessagesProcessor', request.app[MSGS_QUEUE_KEY]
    ).incomingQueue  # Our outgoing queue is the incoming queue of the processor
    
    data = await request.json()
    
    processed: asyncio.Event = asyncio.Event()
    data_received: typing.Optional[types.LoginResponse] = None
    except_received: typing.Optional[Exception] = None
    
    async def callback(data: typing.Any, ex: typing.Optional[Exception]) -> None:
        nonlocal data_received
        nonlocal except_received
        if ex:
            logger.error('Error processing login: %s', ex)
            data_received = None
            except_received = ex
        else:            
            data_received = data
            except_received = None
        processed.set()

    # Append login to process queue
    await outgoingQueue.put(
        types.UDSMessage(
            msg_type=types.UDSMessageType.LOGIN,
            data={
                'username': data['username'],
                'session_type': data.get('session_type', consts.UNKNOWN),
            },
            callback=callback,
        )
    )

    # Wait for login to be processed
    await processed.wait()
    if data_received is None:
        return aiohttp.web.HTTPBadRequest(reason=str(except_received))
    
    return aiohttp.web.json_response(data_received.asDict())
