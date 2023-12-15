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

from . import types

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
    def remove():
        try:
            os.unlink(f.name)
        except Exception:
            pass

    atexit.register(remove)

    return sslContext
