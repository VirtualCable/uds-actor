# -*- coding: utf-8 -*-
#
# Copyright (c) 2024 Virtual Cable S.L.U.
# All rights reserved.
#
# Redistribution and use in source and binary forms, with or without modification,
# are permitted provided that the following conditions are met:
#
#    * Redistributions of source code must retain the above copyright notice,
#      this list of conditions and the following disclaimer.
#    * Redistributions in binary form must reproduce the above copyright notice,
#      this list of conditions and the following disclaimer in the documentation
#      and/or other materials provided with the distribution.
#    * Neither the name of Virtual Cable S.L.U. nor the names of its contributors
#      may be used to endorse or promote products derived from this software
#      without specific prior written permission.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
# AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
# IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
# DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
# FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
# DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
# SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
# CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
# OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
# OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

'''
@author: Adolfo Gómez, dkmaster at dkmon dot com
'''
import logging
import logging.handlers

import servicemanager  # pyright: ignore [reportMissingModuleSource]


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

        try:
            # Convert to own loglevel, basically multiplying by 1000
            if record.levelno >= logging.ERROR:
                servicemanager.LogErrorMsg(msg)
            elif record.levelno >= logging.WARNING:
                servicemanager.LogWarningMsg(msg)
            elif record.levelno >= logging.INFO:
                servicemanager.LogInfoMsg(msg)
        except Exception:  # nosec: If cannot log, just ignore it
            pass

    def __eq__(self, other: object) -> bool:
        """Equality operator.
        Used for testing purposes.
        """
        if not isinstance(other, ServiceLogger):
            return False
        return True
