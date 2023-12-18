#
# (c) 2023 Virtual Cable S.L.U.
#
import typing
import aiohttp.web

CONFIG_KEY: typing.Final[aiohttp.web.AppKey] = aiohttp.web.AppKey('config')
UDSREST_KEY: typing.Final[aiohttp.web.AppKey] = aiohttp.web.AppKey('uds_rest')
MSGS_QUEUE_KEY: typing.Final[aiohttp.web.AppKey] = aiohttp.web.AppKey('msgs_queue')
