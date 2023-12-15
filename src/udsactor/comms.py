import typing
import threading
import asyncio
import secrets

# Event to stop the thread, can be set from other threads
# so it's a threading.Event, not an asyncio.Event
stopEvent: typing.Final[threading.Event] = threading.Event()
# Secret used to authenticate messages from UDS Broker
secret: typing.Final[str] = secrets.token_urlsafe(33)
