# Decorators for the UDS Actor

import typing
import functools
import time
import collections.abc

from udsactor.log import logger

P = typing.ParamSpec('P')
R = typing.TypeVar('R')


# Retries if an exception is raised, sleeping the given time between retries and at most the given number of retries
def retry_on_exception(
    retries: int,
    *,
    wait_seconds: float = 2,
    retryable_exceptions: typing.Optional[typing.List[typing.Type[Exception]]] = None,
    do_log: bool = False,
) -> collections.abc.Callable[[collections.abc.Callable[P, R]], collections.abc.Callable[P, R]]:
    to_retry = retryable_exceptions or [Exception]

    def decorator(fnc: collections.abc.Callable[P, R]) -> collections.abc.Callable[P, R]:
        @functools.wraps(fnc)
        def wrapper(*args: typing.Any, **kwargs: typing.Any) -> R:
            for i in range(retries):
                try:
                    return fnc(*args, **kwargs)
                except Exception as e:
                    if do_log:
                        logger.error('Exception raised in function %s: %s', fnc.__name__, e)

                    if not any(isinstance(e, exception_type) for exception_type in to_retry):
                        raise e

                    # if this is the last retry, raise the exception
                    if i == retries - 1:
                        raise e

                    time.sleep(wait_seconds * (2 ** min(i, 4)))  # Exponential backoff until 16x

            # retries == 0 allowed, but only use it for testing purposes
            # because it's a nonsensical decorator otherwise
            return fnc(*args, **kwargs)

        return wrapper

    return decorator
