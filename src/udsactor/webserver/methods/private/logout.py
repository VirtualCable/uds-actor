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


@routes.post(consts.PRIVATE_REST_LOGOUT)
async def login(request: aiohttp.web.Request) -> aiohttp.web.Response:
    """Processes a launch request (from local)

    The client expects a json response from types.LoginResult
    """
    outgoing_queue: asyncio.Queue = typing.cast(
        'server_msg_processor.MessagesProcessor', request.app[MSGS_QUEUE_KEY]
    ).incoming_queue  # Our outgoing queue is the incoming queue of the processor
    data = await request.json()

    # Append login to process queue
    await outgoing_queue.put(
        types.UDSMessage(
            msg_type=types.UDSMessageType.LOGOUT,
            data={
                'username': data['username'],
                'session_type': data.get('session_type', None),
                'session_id': data.get('session_id', None),
            },
        )
    )

    # For logout, we do not need to wait for response, so we just return OK

    return aiohttp.web.json_response(consts.OK)
