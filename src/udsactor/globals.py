import typing
import threading
import asyncio
import secrets

# Secret used to authenticate messages from UDS Broker
secret: typing.Final[str] = secrets.token_urlsafe(33)
