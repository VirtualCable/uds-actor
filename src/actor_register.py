#!/usr/bin/python3

# Copyright (C) 2024-2025
# Alexander Burmatov
# All rights reserved.
#
# Redistribution and use in source and binary forms, with or without
# modification, are permitted provided that the following conditions are
# met:
#
# * Redistributions of source code must retain the above copyright notice,
#   this list of conditions and the following disclaimer.
# * Redistributions in binary form must reproduce the above copyright notice,
#   this list of conditions and the following disclaimer in the documentation
#   and/or other materials provided with the distribution.
# * Neither the name of the Alexander Burmatov may be used to
#   endorse or promote products derived from this software without
#   specific prior written permission.
#
# THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
# "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
# LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR
# A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT
# OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL,
# SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT
# LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE,
# DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY
# THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
# (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
# OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

"""
@author: Alexander Burmatov, thatman at altlinux dot org
"""
from getpass import getpass
import os

import udsactor


try:
    data: udsactor.types.InterfaceInfoType = next(
        udsactor.platform.operations.getNetworkInfo()
    )

    validate_cert = (
        os.environ.get("OPENUDS_ACTOR_SSL_VALIDATION")
        or input("SSL validation (yes/no): ").lower()
    )
    if validate_cert == "y" or validate_cert == "yes":
        validate_cert = True
    elif validate_cert == "n" or validate_cert == "no":
        validate_cert = False
    else:
        raise Exception(f"SSL validation must be yes/y/no/n")
    hostname = os.environ.get("OPENUDS_HOST") or input("Hostname: ").strip()

    uds_server_api = udsactor.rest.UDSServerApi(hostname, validate_cert)

    auths = list(uds_server_api.enumerateAuthenticators())
    auth_variants = list()
    for a in auths:
        auth_variants.append(a.auth)
    auth_variants.append("admin")
    auth = (
        os.environ.get("OPENUDS_AUTHENTICATOR")
        or input(f"Authenticator {auth_variants}: ").strip()
    )
    if auth not in auth_variants:
        raise Exception(f"Authenticator must be in {auth_variants}")
    username = os.environ.get("OPENUDS_USERNAME") or input("Username: ").strip()
    password = os.environ.get("OPENUDS_PASSWORD") or getpass()
    pre_command = (
        os.environ.get("OPENUDS_ACTOR_PRE_CONNECT") or input("Pre connect: ").strip()
    )
    run_once_command = (
        os.environ.get("OPENUDS_ACTOR_RUN_ONCE") or input("Run once: ").strip()
    )
    post_command = (
        os.environ.get("OPENUDS_ACTOR_POST_CONFIG") or input("Post config: ").strip()
    )
    log_levels = {
        "debug": 0,
        "info": 1,
        "error": 2,
        "fatal": 3,
    }
    log_level = (
        os.environ.get("OPENUDS_ACTOR_LOG_LEVEL")
        or input(f"Log level {list(log_levels.keys())}: ").strip()
    )
    if log_level not in log_levels:
        raise Exception(f"Log level must be in {log_levels.keys()}")
    log_level = log_levels[log_level]

    token = uds_server_api.register(
        auth,
        username,
        password,
        hostname,
        data.ip or "",  # IP
        data.mac or "",  # MAC
        pre_command,
        run_once_command,
        post_command,
        log_level,
    )

    udsactor.platform.store.write_config(
        udsactor.types.ActorConfigurationType(
            actorType=udsactor.types.MANAGED,
            host=hostname,
            validateCertificate=validate_cert,
            master_token=token,
            pre_command=pre_command,
            post_command=post_command,
            runonce_command=run_once_command,
            log_level=log_level,
        )
    )

    print("Registration with UDS completed.")
except udsactor.rest.RESTError as e:
    print(f"UDS Registration error: {e}")