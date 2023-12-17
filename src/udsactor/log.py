import asyncio
import logging
import logging.handlers
import os
import tempfile
import typing
import collections.abc

from . import consts, native, types, utils, rest


class UDSBrokerLogger(metaclass=utils.Singleton):
    """
    Logger that will log to remote log if we are a service
    """

    # Note, once initialized api, it will remain the same for all instances
    # Even if RemoteLogger is reinstanced
    api: 'rest.BrokerREST | rest.PrivateREST |None'
    def_userservice_uuid: "str | None"
    log_queue: asyncio.Queue
    # flag to indicate we are emitting a log message
    # This is used to avoid circular logging
    emitting: bool

    def __init__(self) -> None:
        self.api = None
        self.log_queue = asyncio.Queue()
        self.def_userservice_uuid = None
        self.emitting = False

    @staticmethod
    def manager() -> 'UDSBrokerLogger':
        """
        Returns the singleton instance of this class
        """
        return UDSBrokerLogger()

    async def _log(self, level: int, message: str, userservice_uuid: str | None = None) -> None:
        """
        Logs a message to remote log
        """
        if self.api:
            self.emitting = True
            await self.api.log(
                level=types.LogLevel.fromPython(level), message=message
            )
            self.emitting = False

    @staticmethod
    def setDefaults(userservice_uuid: str | None = None) -> None:
        """
        Sets the default userservice_uuid to be used in all log calls
        """
        UDSBrokerLogger.manager().def_userservice_uuid = userservice_uuid

    @staticmethod
    async def waitAndSendLogs(forever: bool = True) -> None:
        """
        Waits for log messages and logs them

        Will stop when cancelled
        """
        async def process_log(level: int, message: str, userservice_uuid: str | None = None) -> None:
            await UDSBrokerLogger.manager()._log(
                level, message, userservice_uuid or UDSBrokerLogger.manager().def_userservice_uuid
            )
        if forever:
            while True:
                try:
                    level, message, userservice_uuid = await UDSBrokerLogger.manager().log_queue.get()
                    await process_log(level, message, userservice_uuid)
                except asyncio.CancelledError:
                    break
                except Exception:
                    pass
        else:
            try:
                while True:
                    level, message, userservice_uuid = UDSBrokerLogger.manager().log_queue.get_nowait()
                    await process_log(level, message, userservice_uuid)
            except asyncio.QueueEmpty:
                pass

    @staticmethod
    def log(level: int, message: str, userservice_uuid: str | None = None) -> None:
        """
        Logs a message to remote log, using the default userservice_uuid if not specified
        """
        try:
            if not UDSBrokerLogger.manager().emitting:
                UDSBrokerLogger.manager().log_queue.put_nowait((level, message, userservice_uuid))
            # If emitting, we are in a log call, so we cannot log again
        except Exception:
            pass  # Eat exception, we are not interested in it


def setup_log(
    *,
    filename: str | None = None,
    level: str | None = None,
    type: typing.Literal['service', 'config', 'client', 'initial'] = 'initial',
    cfg: types.ActorConfiguration | None = None
) -> None:
    """
    Sets up the logging system

    Args:
        filename: Log file name (defaults to $TEMP/rdsserver.log)
        level: Log level (defaults to INFO)
        type: Type of log (service, config, client, initial)
          - service: We are a service, so we will log to windows event log
          - config: We are a config tool, so we will not log to remote log
          - client: We are a client, so we will log to remote log (in fact, remote "local" log)
          - initial: We are initializing, so we will not log to remote log
        cfg: Configuration to use

    Returns:
        None
    """
    filename = filename or os.path.join(
        tempfile.gettempdir(), ('udsappserver.log' if type == 'server' else 'udsapp.log')
    )

    new_handlers: list[logging.Handler] = [
        logging.handlers.RotatingFileHandler(filename, maxBytes=1024 * 1024 * 10, backupCount=5),
    ]

    # If is a service, add service logger
    if type == 'service':
        logger = native.Manager.instance().logger
        if logger:
            new_handlers.append(logger)
        if cfg:
            setup_remotelog(cfg)
            new_handlers.append(UDSRemoteLogHandler(remote_logger=UDSBrokerLogger.manager()))
    elif type == 'client':
        setup_remotelog(None)
        new_handlers.append(UDSRemoteLogHandler(remote_logger=UDSBrokerLogger.manager()))
    else:
        pass  # For config or initially, there is no remote log

    # If debug feature requested, set level to debug, else to info
    level = (level or 'INFO') if not consts.DEBUG else 'DEBUG'
    levels = {
        'DEBUG': logging.DEBUG,
        'INFO': logging.INFO,
        'WARNING': logging.WARNING,
        'ERROR': logging.ERROR,
        'CRITICAL': logging.CRITICAL,
    }

    print('Logging to {}'.format(filename))
    logging.basicConfig(
        format='%(asctime)s %(levelname)s %(message)s',
        level=levels.get(level, logging.INFO),
        handlers=new_handlers,
        force=True,  # Force to use our handlers
    )


def setup_remotelog(cfg: typing.Optional[types.ActorConfiguration]) -> None:
    """
    Sets up the remote logging system

    Args:
        cfg: Configuration to use

    Returns:
        None
    """
    if cfg:
        UDSBrokerLogger.manager().api = rest.BrokerREST(cfg.host, cfg.validateCertificate, cfg.token)
    else:
        UDSBrokerLogger.manager().api = rest.PrivateREST()


class UDSRemoteLogHandler(logging.Handler):
    """
    Custom log handler for UDS that will log to windows event log if we are a service
    """

    _remote_logger: 'UDSBrokerLogger'

    def __init__(self, remote_logger: 'UDSBrokerLogger') -> None:
        super().__init__()
        self._remote_logger = remote_logger

    def emit(self, record: logging.LogRecord) -> None:
        # To avoid circular imports and loading manager before apps are ready
        # pylint: disable=import-outside-toplevel

        msg = record.getMessage()
        try:
            self._remote_logger.log(
                level=record.levelno,
                message=msg,
            )
        except Exception:
            pass

    def __eq__(self, other: object) -> bool:
        """Equality operator.
        Used for testing purposes.
        """
        if not isinstance(other, UDSRemoteLogHandler):
            return False
        return True  # RemoteLogger is a singleton, so it will always be the same
