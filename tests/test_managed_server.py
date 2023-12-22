#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import sys
import typing
import collections.abc
from unittest import mock

from unittest import IsolatedAsyncioTestCase

from udsactor import native, types, consts

from .utils import tools, fixtures


class TestManagedServer(IsolatedAsyncioTestCase):
    cfg: types.ActorConfiguration

    # Tests the managed version of the server
    def setUp(self) -> None:
        self.cfg = fixtures.configuration(actorType=types.ActorType.MANAGED)

    def tearDown(self) -> None:
        tools.set_testing_cfg(None)


