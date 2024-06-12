#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import atexit
import logging
import os
import ssl
import tempfile

import certifi

from . import types, consts

logger = logging.getLogger(__name__)


def generate_server_ssl_context(certInfo: types.CertificateInfo) -> ssl.SSLContext:
    """Generates a server ssl context"""
    sslContext = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)

    # Save private key + certificate to temp file
    with tempfile.NamedTemporaryFile(mode='w', delete=False) as f:
        f.write(certInfo.key)
        f.write(certInfo.certificate)
        f.flush()
        sslContext.load_cert_chain(f.name, password=certInfo.password)

    # Ensure file is deleted at exit
    def remove() -> None:
        try:
            os.unlink(f.name)
        except Exception:
            pass

    atexit.register(remove)

    return sslContext

def create_client_sslcontext(verify: bool = True) -> ssl.SSLContext:
    """
    Creates a SSLContext for client connections.

    Args:
        verify: If True, the server certificate will be verified. (Default: True)

    Returns:
        A SSLContext object.
    """
    ssl_context = ssl.create_default_context(
        purpose=ssl.Purpose.SERVER_AUTH, cafile=certifi.where()
    )
    if not verify:
        ssl_context.check_hostname = False
        ssl_context.verify_mode = ssl.VerifyMode.CERT_NONE

    # Disable TLS1.0 and TLS1.1, SSLv2 and SSLv3 are disabled by default
    # Next line is deprecated in Python 3.7
    # sslContext.options |= ssl.OP_NO_TLSv1 | ssl.OP_NO_TLSv1_1 | ssl.OP_NO_SSLv2 | ssl.OP_NO_SSLv3
    ssl_context.minimum_version = getattr(
        ssl.TLSVersion, 'TLSv' + consts.SECURE_MIN_TLS_VERSION.replace('.', '_')
    )
    ssl_context.set_ciphers(consts.SECURE_CIPHERS)

    return ssl_context
