#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import typing
import asyncio
import logging

import aiohttp
import aiohttp.web

from ...routes import routes
from ...keys import MSGS_PROCESSOR_KEY

from udsactor import types, native, consts, server_msg_processor
from udsactor.webserver.utils import response

logger = logging.getLogger(__name__)


@routes.post(consts.PUBLIC_REST_PATH('screenshot'))
async def screenshot(request: aiohttp.web.Request) -> aiohttp.web.Response:
    outgoing_queue: asyncio.Queue = typing.cast(
        'server_msg_processor.MessagesProcessor', request.app[MSGS_PROCESSOR_KEY]
    ).queue  # Push the messages to be processed by the processor

    try:
        await outgoing_queue.put(types.UDSMessage(types.UDSMessageType.SCREENSHOT))
    except Exception as e:
        logger.warning('Error processing log: %s', e)
        return response(result=None, error=str(e))

    return response(result=consts.OK)
