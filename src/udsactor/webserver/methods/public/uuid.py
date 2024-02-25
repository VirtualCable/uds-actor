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
from ...keys import CONFIG_KEY

from udsactor import types, consts
from udsactor.webserver.utils import response

logger = logging.getLogger(__name__)


@routes.get(consts.PUBLIC_REST_PATH('uuid'))
async def uuid(request: aiohttp.web.Request) -> aiohttp.web.Response:
    cfg = request.app[CONFIG_KEY]
    return response(result=cfg.token or '' if cfg.actorType == types.ActorType.MANAGED else '')
