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

from udsactor import types, native, consts
from udsactor.webserver.utils import response

logger = logging.getLogger(__name__)


@routes.post(consts.PUBLIC_REST_PATH('preConnect'))
async def preconnect(request: aiohttp.web.Request) -> aiohttp.web.Response:
    return aiohttp.web.json_response(consts.OK)
