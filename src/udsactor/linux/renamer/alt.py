# -*- coding: utf-8 -*-
#
# Copyright (c) 2014-2022 Virtual Cable S.L.U.
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
@author: Alexey Shabalin, shaba at altlinux dot org
'''
import subprocess  # nosec

from .common import renamers
from ...log import logger


def rename(newName: str) -> bool:
    '''
    ALT, ALTLinux, BaseALT Renamer
    Expects new host name on newName
    Host does not needs to be rebooted after renaming
    '''
    logger.debug('using ALT renamer')

    with open('/etc/hostname', 'w') as hostname:
        hostname.write(newName)

    # Force system new name
    subprocess.run(['hostnamectl', 'set-hostname', newName])  # nosec: subprocess
    subprocess.run(['/bin/hostname', newName])  # nosec: subprocess

    # add name to "hosts"
    with open('/etc/hosts', 'r') as hosts:
        lines = hosts.readlines()
    with open('/etc/hosts', 'w') as hosts:
        hosts.write("127.0.1.1\t{}\n".format(newName))
        for l in lines:
            if l[:9] != '127.0.1.1':  # Skips existing 127.0.1.1. if it already exists
                hosts.write(l)

    with open('/etc/sysconfig/network', 'r') as net:
        lines = net.readlines()
    with open('/etc/sysconfig/network', 'w') as net:
        net.write('HOSTNAME={}\n'.format(newName))
        for l in lines:
            if l[:8] != 'HOSTNAME':
                net.write(l)

    return True

# All names in lower case
renamers['altlinux'] = rename
renamers['alt'] = rename
renamers['basealt'] = rename
