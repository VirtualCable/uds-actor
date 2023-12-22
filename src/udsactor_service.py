#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import typing
import asyncio
import logging

from udsactor import log, native, webserver

logger = logging.getLogger(__name__)


def main() -> None:
    log.setup_log(level='INFO')

    manager = native.Manager.instance()
    manager.runner.run()


if __name__ == "__main__":
    main()
