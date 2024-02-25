#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import asyncio
import logging
import typing
import collections.abc

import aiohttp
import aiohttp.web

from udsactor import consts, rest, types, cert, server_msg_processor

# To ensure loading and registering of methods, they have decorators
# that register themselfs on "routes" for aiohttp web server
from . import methods, keys

from .routes import routes

logger = logging.getLogger(__name__)


# Middleware to check token
# Token could be on TOKEN_AUTH Header or on token query parameter
@aiohttp.web.middleware
async def security_checks(
    request: aiohttp.web.Request, handler: collections.abc.Callable[[aiohttp.web.Request], collections.abc.Awaitable[aiohttp.web.StreamResponse]]
) -> aiohttp.web.StreamResponse:
    """
    Several security checks:
        # Checks if the ip is allowed
        Checks if the token is valid
    """
    # cfg = request.app[keys.CONFIG_KEY]

    # Check if ip is allowed. Ips is a list of allowed ips or networks
    # if cfg.allowed_ips:
    #    ip = ipaddress.ip_address(request.remote or '0.0.0.0')
    #    if not any(ip in ipaddress.ip_network(i, strict=False) for i in cfg.allowed_ips):
    #        raise aiohttp.web.HTTPForbidden()

    if request.path == '/':  # Index page does not need token
        return await handler(request)

    # If local request, allow it for launch and stop
    if request.remote in {'127.0.0.1', '::1'}:
        if request.path in consts.VALID_PRIVATE_REST_PATHS:
            return await handler(request)

    # Extract auth token from url if url is "/actor/AUTH_TOKEN/method"
    # where AUTH_TOKEN is the auth token to be checked
    try:
        received_token = request.path.split('/')[2]
        if received_token != consts.OWN_AUTH_TOKEN:
            raise Exception('Invalid token')
    except Exception:
        raise aiohttp.web.HTTPForbidden()

    response = await handler(request)
    # Set server header
    response.headers['Server'] = 'UDSActor/4.0'
    return response


@routes.get('/')
async def index(request: aiohttp.web.Request) -> aiohttp.web.Response:
    """Index page"""
    return aiohttp.web.Response(text=consts.VERSION)


async def server(
    cfg: types.ActorConfiguration,
    cert_info: types.CertificateInfo,
    server_msg_processor: server_msg_processor.MessagesProcessor,
    ready_event: typing.Optional[asyncio.Event] = None,
) -> None:
    """Main server function"""

    # Generate ssl context
    ssl_context = cert.generate_server_ssl_context(cert_info)

    webServer = aiohttp.web.Application(
        logger=logger, middlewares=[security_checks], client_max_size=consts.CLIENT_MAX_SIZE
    )

    # Store some values on webserver data
    webServer[keys.CONFIG_KEY] = cfg
    webServer[keys.UDSREST_KEY] = rest.BrokerREST(
        host=cfg.host, validateCert=cfg.validateCertificate, token=cfg.token
    )
    # This will translate messages from UDS to running actor client
    webServer[keys.MSGS_PROCESSOR_KEY] = server_msg_processor

    webServer.add_routes(routes)
    runner = aiohttp.web.AppRunner(webServer)
    await runner.setup()
    site = aiohttp.web.TCPSite(
        runner=runner,
        host='0.0.0.0',  # Listen on all interfaces, localhost included
        port=consts.LISTEN_PORT,
        ssl_context=ssl_context,
        reuse_address=True,
    )
    await site.start()

    if ready_event is not None:
        ready_event.set()

    logger.debug('Server running...')

    # Wait forever here... (until cancelled)
    try:
        while True:
            await asyncio.sleep(10)
    except asyncio.CancelledError:
        await site.stop()