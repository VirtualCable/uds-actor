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
import asyncio
import typing
import logging

from udsactor import types, consts, log, exceptions, utils
from udsactor.abc import ActorProcessor


logger = logging.getLogger(__name__)


class UnmanagedActorProcessor(ActorProcessor):
    async def initialize(self, interfaces: list[types.InterfaceInfo]) -> typing.Optional[types.CertificateInfo]:
        """
        Processes an unmanaged actor

        Returns:
            None if actor must exit, or a certificate to use on webserver
        """
        logger.info('Starting unmanaged actor')

        cfg = await self.platform.config
        api = await self.api

        # Unmanaged actor simply gets a "registered" certificate to start local swebserver
        return await api.initialize_unmanaged(interfaces, consts.LISTEN_PORT)

    async def initialize_flow_for_unmanaged(self) -> None:
        cfg = await self.platform.config
        api = await self.api

        # We know for sure that we have interfaces, we got them before :)
        interfaces = await self.platform.operations.validNetworkCards()

        # ********************************************
        # Token not transformed, so we must initialize
        # ********************************************
        try:
            # If token was not already transformed, we must initialize
            initResult: types.InitializationResult = await api.initialize(interfaces, cfg.actorType)

            # Unmanaged actor has to keep local token, even with the ono provided by server
            # This is so because same machine will be used by different user services
            # So we will not save it, and after user logout, we will reload config (restoring token)
            if initResult.token and cfg.token != initResult.token:
                logger.debug('Token changed from %s to %s', cfg.token, initResult.token)
                cfg = cfg._replace(token=initResult.token, initialized=True)

            # Store config
            cfg = cfg._replace(
                config=types.ActorDataConfiguration(unique_id=initResult.unique_id, os=initResult.os)
            )
            logger.debug('Config not saved, only updated: %s', cfg)
        except (exceptions.RESTConnectionError, exceptions.RESTError) as e:
            logger.warning('Error validating with Broker: %s', e)
        # Remember, cancel will raise a CancelledError exception and do not enter here
        except Exception as e:
            logger.error('Unknown error: %s', e)

        # Setup remote logging now that we have a valid token
        log.setup_log(type='service', cfg=cfg)

    async def login(self, username: str, sessionType: str) -> types.LoginResponse:
        cfg = await self.platform.config
        api = await self.api

        # First, invoke "partial" initialization for unmanaged actor
        # This will setup our token and other things
        await self.initialize_flow_for_unmanaged()

        result = types.LoginResponse(ip='', hostname='', dead_line=None, max_idle=None, session_id=None)
        try:
            result = await api.notify_login(
                actor_type=types.ActorType.UNMANAGED, username=username, session_type=sessionType
            )
            script = await self.platform.cfgManager.scriptToInvokeOnLogin()
            if script:
                logger.info('Executing script on login: {}'.format(script))
                script += f'{username} {sessionType or "unknown"} {cfg.actorType}'
                await utils.execute(script, 'Logon')
        except exceptions.RESTError as e:
            logger.error('Error notifying login: %s', e)

        return result

    async def logout(self, username: str, session_type: str, session_id: str) -> None:
        cfg = await self.platform.config
        api = await self.api

        try:
            await api.notify_logout(
                actor_type=types.ActorType.UNMANAGED,
                username=username,
                session_type=session_type,
                session_id=session_id,
            )
        except exceptions.RESTError as e:
            logger.error('Error notifying logout: %s', e)

        # Ensure token is resetted to original value
        await self.platform.clear_config()
