# Decorators for the UDS Actor

import typing
import functools
import time

from udsactor.log import logger

FT = typing.TypeVar('FT', bound=typing.Callable[..., typing.Any])


# Retries if an exception is raised, sleeping the given time between retries and at most the given number of retries
def retry_on_exception(
    retries: int,
    *,
    wait_seconds: float = 2,
    retryable_exceptions: typing.Optional[typing.List[typing.Type[Exception]]] = None,
    do_log: bool = False,
) -> typing.Callable[[FT], FT]:
    def decorator(func: FT) -> FT:
        @functools.wraps(func)
        def wrapper(*args, **kwargs) -> typing.Any:
            for i in range(retries):
                try:
                    return func(*args, **kwargs)
                except Exception as e:
                    if do_log:
                        logger.error('Exception raised in function %s: %s', func.__name__, e)

                    # If the exception is not in the list of exceptions to retry, raise it
                    retryable_exceptions = retryable_exceptions or [Exception]

                    if not any(isinstance(e, exception_type) for exception_type in retryable_exceptions):
                        raise e

                    # if this is the last retry, raise the exception
                    if i == retries - 1:
                        raise e

                    time.sleep(wait_seconds)

        return typing.cast(FT, wrapper)

    return decorator
