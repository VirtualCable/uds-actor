#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import logging
import typing

import asyncio
import aiohttp
import aiohttp.web

from udsactor import consts, globals, types

from ...routes import routes
from ...keys import MSGS_QUEUE_KEY

logger = logging.getLogger(__name__)

if typing.TYPE_CHECKING:
    from udsactor import rest, server_msg_processor


@routes.post(consts.PRIVATE_REST_LOG)
async def log(request: aiohttp.web.Request) -> aiohttp.web.Response:
    """Processes a log request (from local)"""

    outgoingQueue: asyncio.Queue = typing.cast(
        'server_msg_processor.MessagesProcessor', request.app[MSGS_QUEUE_KEY]
    ).incomingQueue  # Our outgoing queue is the incoming queue of the processor

    data = await request.json()

    try:
        data = await request.json()
        await outgoingQueue.put(
            types.UDSMessage(
                msg_type=types.UDSMessageType.LOG,
                data=data,
            )
        )
    except Exception as e:
        logger.warning('Error processing log: %s', e)
        raise aiohttp.web.HTTPBadRequest(reason=f'Launch error: {e}')
    return aiohttp.web.json_response(consts.OK)
