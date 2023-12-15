import logging
import logging.handlers


class ServiceLogger(logging.Handler):
    """
    Custom log handler for UDS that will log to windows event log if we are a service
    """

    def __init__(
        self,
    ) -> None:
        super().__init__()

    def emit(self, record: logging.LogRecord) -> None:
        msg = f'{record.levelname} {record.getMessage()}'


    def __eq__(self, other: object) -> bool:
        """Equality operator.
        Used for testing purposes.
        """
        if not isinstance(other, ServiceLogger):
            return False
        return True
