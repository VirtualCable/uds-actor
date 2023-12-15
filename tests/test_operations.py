#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import socket
import os
import typing
import collections.abc

from unittest import IsolatedAsyncioTestCase

from udsactor import platform, types, consts

from .utils.tools import rnd_string_for_test


class TestOperations(IsolatedAsyncioTestCase):
    operations: 'platform.abc.Operations'

    def setUp(self) -> None:
        self.operations = platform.Platform.platform().operations

    async def test_user_is_admin(self) -> None:
        self.assertIn(await self.operations.IsUserAnAdmin(), (True, False))

    async def test_get_computer_name(self) -> None:
        self.assertEqual((await self.operations.getComputerName()).lower(), socket.gethostname().lower())

    async def test_network_info(self) -> None:
        netInfo = await self.operations.getNetworkInfo()
        self.assertIsInstance(netInfo, list)
        self.assertGreater(len(netInfo), 0)
        self.assertIsInstance(netInfo[0], types.InterfaceInfo)

    async def test_get_domain_name(self):
        self.assertIsInstance(await self.operations.getDomainName(), (str, type(None)))

    async def test_get_windows_version(self) -> None:
        self.assertIsInstance(await self.operations.getOSVersion(), str)
        self.assertIsInstance(await self.operations.getOSName(), str)

    # reboot, logout, changeUserPassword, joinDomain, rename, ... are not tested, for obvious reasons :-)

    async def test_idle(self) -> None:
        self.assertEqual(await self.operations.initIdleDuration(32), None)
        idle = await self.operations.getIdleDuration()
        self.assertIsInstance(idle, float)
        self.assertGreaterEqual(idle, 0)  # If executed from a console, idle will be 0

    async def test_get_current_user(self) -> None:
        self.assertIsInstance(await self.operations.getCurrentUser(), str)

    async def test_get_session_type(self) -> None:
        self.assertIsInstance(await self.operations.getSessionType(), str)
