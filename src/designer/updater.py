#!/usr/bin/env python3
# -*- coding: utf-8 -*-
#
# Copyright (c) 2023 Virtual Cable S.L.U.
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
#    * Neither the name of Virtual Cable S.L. nor the names of its contributors
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
Author: Adolfo Gómez, dkmaster at dkmon dot com
'''

# This script compiles the Qt Designer files into Python files and
# makes sure that the generated files from Qt Designer are compatible with both PySide6 and PyQt5

import typing
import subprocess
import re
import logging

REGEX_UI = r'(from PySide6[^\n]*import \(.*?\))\nfrom \.'
REGEX_RCC = r'(from PySide6 import QtCore\n)'

# Basic init logging stuff to stdout
logging.basicConfig(level=logging.DEBUG, format='%(asctime)s %(levelname)s %(message)s')

OUTPUT_DIALOG_UI: typing.Final[str] = '../ui/setup_dialog_ui.py'
OUTPUT_DIALOG_UNMANAGED_UI: typing.Final[str] = '../ui/setup_dialog_unmanaged_ui.py'
OUTPUT_RC: typing.Final[str] = '../ui/uds_rc.py'

# New actor must support PyQt5 and PySide6 (PyQt5 for older operating systems, PySide6 for newer ones)

def patch(regex: str, file: str) -> None:
    with open(file, 'r') as f:
        content = f.read()
        # Look for first match
        match = re.search(regex, content, re.DOTALL | re.MULTILINE)
        if match:
            data = match.group(1)
            # Indent the data
            data = '    ' + data.replace('\n', '\n    ').rstrip()
            # Replace PySide6 with PyQt5
            data2 = data.replace('PySide6', 'PyQt5')
            # Generate a Try: ... Except: ... block
            try_block = f'try:\n{data}\nexcept ImportError:\n{data2}\n'
            # Replace the match with the try block
            content = content.replace(match.group(1), try_block)
        else:
            raise Exception(f'No match found for {regex} in {file}')
    with open(file, 'w') as f:
        f.write(content)


def main() -> None:
    # pyside6-uic setup-dialog.ui -o ../ui/setup_dialog_ui.py --from-imports
    subprocess.run(['pyside6-uic', 'setup-dialog.ui', '-o', OUTPUT_DIALOG_UI, '--from-imports'], check=True)
    # Patch the generated file to use PyQt5 if PySide6 is not available
    patch(REGEX_UI, OUTPUT_DIALOG_UI)
    
    # pyside6-uic setup-dialog-unmanaged.ui -o ../ui/setup_dialog_unmanaged_ui.py --from-imports
    subprocess.run(['pyside6-uic', 'setup-dialog-unmanaged.ui', '-o', OUTPUT_DIALOG_UNMANAGED_UI, '--from-imports'], check=True)
    # Patch the generated file to use PyQt5 if PySide6 is not available
    patch(REGEX_UI, OUTPUT_DIALOG_UNMANAGED_UI)

    # pyside6-rcc uds.qrc -o ../ui/uds_rc.py
    subprocess.run(['pyside6-rcc', 'uds.qrc', '-o', OUTPUT_RC], check=True)
    # Patch the generated file to use PyQt5 if PySide6 is not available
    patch(REGEX_RCC, OUTPUT_RC)


if __name__ == "__main__":
    main()
