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

from udsactor import globals, types, consts, log, exceptions, utils
from udsactor.abc import ActorProcessor


logger = logging.getLogger(__name__)


class ManagedActorProcessor(ActorProcessor):
    async def initialize(self, interfaces: list[types.InterfaceInfo]) -> typing.Optional[types.CertificateInfo]:
        """
        Processes a managed actor

        Returns:
            None if actor must exit, or a certificate to use on webserver
        """
        logger.info('Starting managed actor')

        cfg = await self.config
        api = await self.api

        # ********************************************
        # Token not transformed, so we must initialize
        # ********************************************
        if cfg.initialized is False:
            while True:
                try:
                    # If token was not already transformed, we must initialize
                    initResult: types.InitializationResult = await api.initialize(interfaces, cfg.actorType)

                    if initResult.token:
                        # Replace token with the one provided by server
                        if initResult.token != cfg.token:
                            logger.debug('Token changed from %s to %s', cfg.token, initResult.token)
                            cfg = cfg._replace(token=initResult.token, initialized=True)
                            # And store it

                    # Store config
                    cfg = cfg._replace(
                        config=types.ActorDataConfiguration(unique_id=initResult.unique_id, os=initResult.os)
                    )
                    await self.platform.cfgManager.write(cfg)
                except (exceptions.RESTConnectionError, exceptions.RESTError) as e:
                    logger.warning('Error validating with Broker: %s', e)
                # Remember, cancel will raise a CancelledError exception and do not enter here
                except Exception as e:
                    logger.error('Unknown error: %s', e)
                else:
                    break  # If no error, break loop
                await asyncio.sleep(consts.WAIT_RETRY)  # Wait a bit before retrying

        # Setup remote logging now that we have a valid token
        log.setup_log(type='service', cfg=cfg)

        # **********************************************************
        # Connection Initialized, proceed to configure local machine
        # **********************************************************

        # First, if runOnce command is requested, run it, cleand it and exit
        if cfg.runonce_command:
            runOnce = cfg.runonce_command
            # Remove from cfg and save it
            cfg = cfg._replace(runonce_command=None)
            await self.platform.cfgManager.write(cfg)

            if utils.execute(runOnce, "runOnce"):
                # If runonce is present, will not do anythin more
                # So you must ensure that, when runonce command is finished, reboots the machine.
                # That is, the COMMAND itself has to restart the machine!
                # And tnis is mandatory, because the service will stop after this command is executed
                return None

        # If error executing runonce, we will continue configuration as if nothing happened

        # Configure machine
        retries = consts.RETRIES * 4  # 4 times more retries here...
        while True:
            try:
                if cfg.config and cfg.config.os:
                    # Note that "osData" is "osManager related data"
                    # It will be provided on first initialization, and will be stored on cfg
                    osData = cfg.config.os
                    custom = osData.custom or {}  # If no custom data, nothing to do...

                    # If already done, skip
                    if custom.get('udsdone', False):
                        logger.info('Configuration already done, skipping')
                        return None

                    if osData.action == 'rename':
                        if self.platform.operations.hloRename(
                            osData.name,
                            custom.get('username'),
                            custom.get('password'),
                            custom.get('new_password'),
                        ):
                            # If returns true, a reboot is needed
                            await self.platform.operations.reboot()
                            return None  # No more to do here
                    elif osData.action == 'rename_ad':
                        if await self.platform.operations.hloJoinDomain(osData.name, custom):
                            # If returns true, a reboot is needed
                            await self.platform.operations.reboot()
                break  # If no error, break loop
            except Exception as e:
                retries -= 1
                logger.error('Error configuring machine: %s (retry left %s)', e, retries)
                if retries == 0:  # Reboot and exit
                    logger.info('Rebooting machine for recovery')
                    await self.platform.operations.reboot()
                    return None
                await asyncio.sleep(consts.WAIT_RETRY)

        # If we are here, configuration is done, clean up os configuration data (for security)
        # (clenup is done by removing os data and custom data from cfg)
        cfg = cfg._replace(
            config=typing.cast(types.ActorDataConfiguration, cfg.config)._replace(os=None), data=None
        )
        await self.platform.cfgManager.write(cfg)

        # If "post config" command is present, execute it now
        # Note that this command is executed only once, and will be removed from cfg after execution
        # Also, this commad differs from "runonce" in that control is expected to return to this point
        if cfg.post_command:
            await utils.execute(cfg.post_command, 'postConfig')
            cfg = cfg._replace(post_command=None)
            await self.platform.cfgManager.write(cfg)

        # Look for service interface
        try:
            serviceInterface = next(
                x
                for x in interfaces
                if x.mac.lower() == typing.cast(types.ActorDataConfiguration, cfg.config).unique_id
            )
        except StopIteration:
            serviceInterface = interfaces[0]

        # Notify broker of readyness
        retries = consts.RETRIES * 10  # 10 times more retries here...
        connectionErrorAlreadyLogged = False
        certificate: typing.Optional[types.CertificateInfo] = None
        while True:
            try:
                certificate = await api.ready(
                    globals.secret,
                    serviceInterface.ip,
                    consts.LISTEN_PORT,
                )
            except exceptions.RESTError as e:
                # If we get an error here, we will retry, but log it only once
                if not connectionErrorAlreadyLogged:
                    logger.warning('Error notifying broker of readyness: %s', e)
                    connectionErrorAlreadyLogged = True
                retries -= 1
            except Exception as e:
                # If we get an error here, we will retry, but log it only once
                logger.error('Unmanaged error notifying broker of readyness: %s', e)
                retries -= 1
            else:
                break

            if retries == 0:
                logger.error('Could not notify broker of readyness, rebooting')
                await self.platform.operations.reboot()
                return None

            await asyncio.sleep(consts.WAIT_RETRY)

        return certificate

    async def login(self, username: str, sessionType: str) -> types.LoginResultInfo:
        cfg = await self.config
        api = await self.api

        result = types.LoginResultInfo(ip='', hostname='', dead_line=None, max_idle=None, session_id=None)
        try:
            result = await api.notify_login(
                actor_type=cfg.actorType or types.ActorType.MANAGED, username=username, session_type=sessionType
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
