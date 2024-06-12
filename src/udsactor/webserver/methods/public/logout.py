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


@routes.post(consts.PUBLIC_REST_PATH('logout'))
async def logout(request: aiohttp.web.Request) -> aiohttp.web.Response:
    queue = request.app[MSGS_PROCESSOR_KEY].queue  # Push the messages to be processed by the processor
    
    await queue.put(
        types.UDSMessage(
            msg_type=types.UDSMessageType.LOGOUT,
            data=types.LogoutRequest.null(from_broker=True).as_dict(),
        )
    )
    return response(result=consts.OK)
