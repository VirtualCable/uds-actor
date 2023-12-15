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

from udsactor import types, platform, consts
from udsactor.webserver.utils import response

logger = logging.getLogger(__name__)


@routes.post(consts.PUBLIC_REST_PATH('logout'))
async def logout(request: aiohttp.web.Request) -> aiohttp.web.Response:
    return response(result=consts.OK)
