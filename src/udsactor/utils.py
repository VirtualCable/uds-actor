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
import contextlib
import ipaddress

from . import types, consts

logger = logging.getLogger(__name__)

# For type checking generics
T = typing.TypeVar('T')
AsyncCallable = collections.abc.Callable[..., collections.abc.Awaitable[typing.Any]]
FT = typing.TypeVar('FT', bound=AsyncCallable)


HUMAN_SYMBOLS: typing.Final[list[str]] = ['B', 'K', 'M', 'G', 'T', 'P', 'E', 'Z', 'Y']
HUMAN_PREFIXS: typing.Final[dict[str, int]] = {
    s: 1 << (i + 1) * 10 for i, s in enumerate(('K', 'M', 'G', 'T', 'P', 'E', 'Z', 'Y'))
}


def bytes2human(n: int, *, format: str = "{value:.0f}{symbol:s}") -> str:
    """
    http://goo.gl/zeJZl
    """
    for symbol in reversed(HUMAN_SYMBOLS[1:]):
        if n >= HUMAN_PREFIXS[symbol]:
            value = float(n + 1) / HUMAN_PREFIXS[symbol]
            return format.format(value=value, symbol=symbol)
    return format.format(value=n, symbol=HUMAN_SYMBOLS[0])


def random_string(length: int = 32) -> str:
    return ''.join(random.SystemRandom().choices(string.ascii_letters + string.digits, k=length))


def ensure_ticket_is_ok(ticketId: str) -> str:
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


def ensure_list(value: typing.Any) -> list[typing.Any]:
    """
    Ensures that the value is a list

    Args:
        value: Value to check

    Returns:
        List
    """
    if isinstance(value, list):
        return value  # pyright: ignore  # This is a known list
    if isinstance(value, collections.abc.Iterable):
        return list(value)  # pyright: ignore  # This is a known list from iterable of ANY
    return [value]


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
) -> collections.abc.Callable[[FT], FT]:
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

    def cache_decorator(fnc: FT) -> FT:
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

        async def wrapper(*args: typing.Any, **kwargs: typing.Any) -> typing.Any:
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

        return typing.cast(FT, wrapper)

    return cache_decorator


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


def retry(times: int = 3, initial_delay: float = 8.0) -> collections.abc.Callable[[FT], FT]:
    """Decorator that will execute method N times, with an increasing delay between executions of the method
    Args:
        times: Number of times to retry
        initial_delay: Initial delay between retries (will be doubled on each retry), default 8 seconds

    Example:
        @retry(times=3, initial_delay=1)
    """

    def retry_decorator(fnc: FT) -> FT:
        async def wrapper(
            *args: typing.Any, **kwargs: typing.Any
        ) -> typing.Any:  # Will always return something or raise an exception
            for i in range(times):
                try:
                    return await fnc(*args, **kwargs)
                except Exception as e:
                    logger.warning('Exception %s on %s, retrying %s more times', e, fnc, times - i - 1)
                    if i == times - 1:
                        raise
                    await asyncio.sleep(initial_delay * (2 << i))

        return typing.cast(FT, wrapper)

    return retry_decorator


async def execute(command_line: str, section: str) -> bool:
    '''Executes an external command

    Args:
        cmdLine: Command line to execute
        section: Section to log

    Returns:
        True if command was executed, False otherwise (and error is logged)
    '''
    try:
        logger.debug('Executing command on {}: {}'.format(section, command_line))
        res = await asyncio.create_subprocess_shell(
            command_line, stdout=asyncio.subprocess.PIPE, stderr=asyncio.subprocess.PIPE
        )
        _stdout, _stderr = await res.communicate()  # Extract stdout and stderr
    except Exception as e:
        logger.error('Got exception executing: {} - {} - {}'.format(section, command_line, e))
        return False
    logger.debug('Result of executing cmd for {} was {}'.format(section, res))
    return True


async def script_executor(script: str) -> None:
    '''
    Executes a script in a thread

    Note:
        * We execute the script in a different thread because it may block for a long time
        * Note tha if the script is async, will need its own event loop in order to work properly
        * If you need to make it async, use asyncio.run() inside the script
        * Normally, due to the fact that it is executed in a different thread, no need for making it async

    Args:
        script: Script to execute (python code)

    Returns:
        Nothing (if error, it is logged)
    '''

    def executor() -> None:
        try:
            exec(script, globals(), None)
        except Exception as e:
            logger.error('Error executing script: {}'.format(e))

    # Execute in a thread
    await asyncio.get_event_loop().run_in_executor(None, executor)


# Singleton metaclass
class Singleton(type):
    '''
    Metaclass for singleton pattern
    Usage:

    class MyClass(metaclass=Singleton):
        ...
    '''

    _instance: typing.Optional['Singleton'] = None

    # We use __init__ so we customise the created class from this metaclass
    def __init__(
        cls: 'type[Singleton]', name: str, bases: typing.Tuple[type, ...], dct: dict[str, typing.Any]
    ) -> None:
        cls._instance = None  # Initial value

    def __call__(cls: 'type[Singleton]', *args: typing.Any, **kwargs: typing.Any) -> typing.Any:
        if cls._instance is None:
            cls._instance = super().__call__(*args, **kwargs)
        return cls._instance
