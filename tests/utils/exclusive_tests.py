import threading
from unittest import IsolatedAsyncioTestCase


from udsactor import consts, globals, types, rest, managed

# Test can be run multithreaded, so we need to ensure that only one test is run at a time
# for these tests
# Also, due to the fact that there are more than one event loop, we need to ensure that
# the test is run alone and comms.Queue is not shared between event loops
# Also, the configuration is keep in a singleton global variable, so we need to ensure
# that those kind of tests are run alone
testLock = threading.Lock()

class AsyncExclusiveTests(IsolatedAsyncioTestCase):
    async def asyncSetUp(self) -> None:
        await super().asyncSetUp()
        testLock.acquire()
        
    async def asyncTearDown(self) -> None:
        await super().asyncTearDown()
        testLock.release()
