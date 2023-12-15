"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import random
import typing
import collections.abc
import logging
import string
import functools
import time
import asyncio
import re
import datetime
import random
import contextlib
import logging
import ipaddress

from . import types, consts

logger = logging.getLogger(__name__)

# For type checking generics
T = typing.TypeVar('T')
Coro = collections.abc.Coroutine[None, None, typing.Any]


def bytes2human(n: int, *, format: str = "{value:.0f}{symbol:s}") -> str:
    """
    http://goo.gl/zeJZl
    """
    symbols = ('B', 'K', 'M', 'G', 'T', 'P', 'E', 'Z', 'Y')
    prefix = {}
    for i, s in enumerate(symbols[1:]):
        prefix[s] = 1 << (i + 1) * 10
    for symbol in reversed(symbols[1:]):
        if n >= prefix[symbol]:
            value = float(n + 1) / prefix[symbol]
            return format.format(value=value, symbol=symbol)
    return format.format(value=n, symbol=symbols[0])


def random_string(length: int = 32) -> str:
    return ''.join(random.SystemRandom().choices(string.ascii_letters + string.digits, k=length))


def ensureTicketIsOk(ticketId: str) -> str:
    """Ensures that ticket is ok, and returns it if so"""
    if len(ticketId) != 40 or re.match(r'^[a-zA-Z0-9]+$', ticketId) is None:
        raise Exception('Invalid ticket: {}'.format(ticketId))
    return ticketId


def ensure_valid_uuid(uuid: str) -> str:
    """
    Very simple security check

    Will ensure that uuid contains only A-Z, a-z, 0-9, -

    Args:
        uuid: UUID to check

    Returns:
        "Checked" uuid
    """
    return re.sub('[^a-zA-Z0-9-]', '', uuid[:32])


def ensure_list(value: typing.Union[T, list[T]]) -> list[T]:
    """
    Ensures that the value is a list

    Args:
        value: Value to check

    Returns:
        List
    """
    if not isinstance(value, list):
        return [value]
    return value


def str_to_net(
    net: typing.Optional[str],
) -> typing.Optional[ipaddress.IPv4Network | ipaddress.IPv6Network]:
    """Converts a string to a network

    Args:
        net: Network to convert (string)

    Returns:
        Network or None if not valid
    """
    if not net:  # Empty or None
        return None

    try:
        return ipaddress.IPv4Network(net)
    except Exception:
        pass  # Better do not anidate try/except

    try:
        return ipaddress.IPv6Network(net)
    except Exception:
        pass

    return None  # Invalid network, return None


def ip_in_net(
    ip: typing.Union[ipaddress.IPv4Address, ipaddress.IPv6Address, str],
    net: typing.Union[ipaddress.IPv4Network, ipaddress.IPv6Network, str],
) -> bool:
    """Checks if an ip is inside a network

    Args:
        ip: IP to check
        net: Network to check against

    Returns:
        True if ip is inside net, False otherwise
    """
    if not ip or not net:  # Empty or None
        return False

    if isinstance(ip, str):
        try:
            ip = ipaddress.ip_address(ip)
        except Exception:
            return False

    if isinstance(net, str):
        try:
            net = ipaddress.ip_network(net)
        except Exception:
            return False

    return ip in net


class ByPassCache(Exception):
    """
    Exception to be raised by a function to bypass the cache
    Argument 0 will be the value to return

    Example:
        @async_lru_cache()
        async def myfunc():
            ...
            raise ByPassCache('This will be returned, but not cached (cache will be bypassed and cleared (if present))')
    """

    pass


# lru_cache for async functions
def async_lru_cache(
    maxsize: int = 128,
    maxduration: typing.Optional[types.CacheDuration] = None,
    ignore_args: typing.Optional[typing.Iterable[int]] = None,
    ignore_kwargs: typing.Optional[typing.Iterable[str]] = None,
):
    """
    Least-recently-used cache decorator for async functions.

    Args:
        maxsize: Max size of the cache
        maxduration: Max duration of the cache
        ignore_args: List of arguments to ignore when caching (by index, first argument is 1)
        ignore_kwargs: List of arguments to ignore when caching (by name)
    Returns:
        Decorator
    """

    def decorator(
        fnc: collections.abc.Callable[..., collections.abc.Coroutine[None, None, T]]
    ) -> collections.abc.Callable[..., collections.abc.Coroutine[None, None, T]]:
        skip_kwargs_set = set(ignore_kwargs or [])
        skip_args_set = set(ignore_args or [])
        duration = maxduration or datetime.timedelta(days=999999)  # A big enough duration

        lock = asyncio.Lock()
        hits = misses = 0

        class CachedItem(typing.NamedTuple):
            timestamp: int
            value: typing.Any

        cache: dict[tuple[tuple[str, ...], frozenset[tuple[str, str]]], CachedItem] = {}

        duration_seconds = duration.total_seconds()

        @functools.wraps(fnc)
        async def wrapper(*args, **kwargs) -> T:
            nonlocal hits, misses
            skip_caching = False
            async with lock:
                # Check if "no_cache" is present in kwargs. Does not matter if it's True or False, just present
                if 'no_cache' in kwargs:
                    del kwargs['no_cache']  # Remove it
                    skip_caching = True

                # Remove skipped args for key
                kargs = tuple([str(v) for i, v in enumerate(args) if i + 1 not in skip_args_set])
                # Remove skipped kwargs for key
                kkwargs = {str(k): str(v) for k, v in kwargs.items() if k not in skip_kwargs_set}

                key = (kargs, frozenset(kkwargs.items()))

                now = int(time.time())
                # Cache hit
                if not skip_caching:
                    if key in cache:
                        item = cache[key]
                        if item.timestamp + duration_seconds > now:
                            hits += 1
                            return item.value
                        else:
                            del cache[key]  # Expired
                    # Cache miss
                    misses += 1
                # Make room if necessary for the new item
                if len(cache) >= maxsize:
                    # Remove the oldest item
                    oldest_key = min(cache, key=lambda k: cache[k].timestamp)
                    del cache[oldest_key]

            # Add the new item, lock released
            try:
                result = await fnc(*args, **kwargs)

                async with lock:
                    cache[key] = CachedItem(timestamp=now, value=result)
                return result
            except ByPassCache as e:
                async with lock:
                    if key in cache:
                        del cache[key]
                if e.args:
                    return e.args[0]
                raise ValueError('ByPassCache exception must have a value')
            except Exception as e:
                async with lock:
                    if key in cache:
                        del cache[key]
                raise e

        async def cache_info() -> types.CacheInfo:
            """Report cache statistics"""
            async with lock:
                return types.CacheInfo(hits, misses, maxsize, len(cache))

        async def cache_clear() -> None:
            """Clear the cache and cache statistics"""
            nonlocal hits, misses
            async with lock:
                cache.clear()
                hits = misses = 0

        # Same as lru_cache
        wrapper.cache_info = cache_info  # type: ignore
        wrapper.cache_clear = cache_clear  # type: ignore
        wrapper.maxsize = maxsize  # type: ignore
        wrapper.maxduration = maxduration  # type: ignore

        return wrapper

    return decorator


@contextlib.asynccontextmanager
async def logexceptions(message: str, *, backtrace: bool = False) -> collections.abc.AsyncGenerator[None, None]:
    """
    Context manager to log exceptions

    Usage:
        async with logexceptions():
            # Code that may raise exceptions

    Args:
        message: Message to log
        backtrace: If true, will log backtrace of exception
    """
    try:
        yield
    except asyncio.CancelledError:  # Since 3.8, CancelledError is not an Exception, but a BaseException
        raise
    except Exception as e:
        if backtrace:
            logger.exception(message)
        else:
            logger.error('%s: %s', message, e)


def retry(
    times: int = consts.RETRIES, initial_delay: typing.Union[int, float] = consts.WAIT_RETRY
) -> collections.abc.Callable[[collections.abc.Callable[..., Coro]], collections.abc.Callable[..., Coro]]:
    """Decorator that will execute method N times, with an increasing delay between executions of the method
    Args:
        times: Number of times to retry
        initial_delay: Initial delay between retries (will be doubled on each retry), default 8 seconds

    Example:
        @retry(times=3, initial_delay=1)
    """

    def decorator(fnc: collections.abc.Callable[..., Coro]) -> collections.abc.Callable[..., Coro]:
        @functools.wraps(fnc)
        async def wrapper(*args, **kwargs) -> T:  # type: ignore  # Will always return something or raise an exception
            for i in range(times):
                try:
                    return await fnc(*args, **kwargs)
                except Exception as e:
                    logger.warning('Exception %s on %s, retrying %s more times', e, fnc, times - i - 1)
                    if i == times - 1:
                        raise
                    await asyncio.sleep(float(initial_delay) * (2 << i))

        return wrapper

    return decorator


async def execute(cmdLine: str, section: str) -> bool:
    try:
        logger.debug('Executing command on {}: {}'.format(section, cmdLine))
        res = await asyncio.create_subprocess_shell(
            cmdLine, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
        )
        stdout, stderr = await res.communicate()
    except Exception as e:
        logger.error('Got exception executing: {} - {} - {}'.format(section, cmdLine, e))
        return False
    logger.debug('Result of executing cmd for {} was {}'.format(section, res))
    return True


# Singleton metaclass
class Singleton(type):
    '''
    Metaclass for singleton pattern
    Usage:

    class MyClass(metaclass=Singleton):
        ...
    '''

    _instance: typing.Optional[typing.Any]

    # We use __init__ so we customise the created class from this metaclass
    def __init__(self, *args, **kwargs) -> None:
        self._instance = None
        super().__init__(*args, **kwargs)

    def __call__(self, *args, **kwargs) -> typing.Any:
        if self._instance is None:
            self._instance = super().__call__(*args, **kwargs)
        return self._instance
