import logging
import logging.handlers


class ServiceLogger(logging.Handler):
    """
    Custom log handler for UDS that will log to linux system log
    """

    def __init__(
        self,
    ) -> None:
        super().__init__()

    def emit(self, record: logging.LogRecord) -> None:
        _msg = f'{record.levelname} {record.getMessage()}'
        # TODO: Log to linux system log


    def __eq__(self, other: object) -> bool:
        """Equality operator.
        Used for testing purposes.
        """
        if not isinstance(other, ServiceLogger):
            return False
        return True
