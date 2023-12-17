#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import typing
import logging

import aiohttp
import aiohttp.web

from ...routes import routes

from udsactor import types, native, consts
from udsactor.webserver.utils import response

logger = logging.getLogger(__name__)


@routes.get(consts.PUBLIC_REST_PATH('screenshot'))
async def screenshot(request: aiohttp.web.Request) -> aiohttp.web.Response:
    return aiohttp.web.json_response(consts.OK)
