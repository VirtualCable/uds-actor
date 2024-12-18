#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import logging

import aiohttp
import aiohttp.web

from ...routes import routes
from ...keys import MSGS_PROCESSOR_KEY

from udsactor import types, consts
from udsactor.webserver.utils import response


logger = logging.getLogger(__name__)


@routes.post(consts.PUBLIC_REST_PATH('preconnect'))
@routes.post(consts.PUBLIC_REST_PATH('preConnect'))  # Old name, for compatibility
async def preconnect(request: aiohttp.web.Request) -> aiohttp.web.Response:
    queue = request.app[MSGS_PROCESSOR_KEY].queue  # Push the messages to be processed by the processor

    try:
        data = types.PreconnectRequest.from_dict(await request.json())
        await queue.put(types.UDSMessage(types.UDSMessageType.PRECONNECT, data=data.as_dict()))
    except Exception as e:
        logger.warning('Error processing log: %s', e)
        return response(result=None, error=str(e))

    return response(result=consts.OK)