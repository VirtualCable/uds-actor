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

from udsactor import types, consts
from udsactor.webserver.utils import response

logger = logging.getLogger(__name__)


@routes.post(consts.PUBLIC_REST_PATH('message'))
async def message(request: aiohttp.web.Request) -> aiohttp.web.Response:
    queue: 'asyncio.Queue[types.UDSMessage]' = typing.cast(
        'asyncio.Queue[types.UDSMessage]', request.app[MSGS_PROCESSOR_KEY]
    )
    try:
        data = await request.json()
        message = data['message']
        await queue.put(types.UDSMessage(types.UDSMessageType.MESSAGE, data=message))
    except Exception as e:
        logger.warning('Error processing log: %s', e)
        return response(result=None, error=str(e))

    return response(result=consts.OK)
