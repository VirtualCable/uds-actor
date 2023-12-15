#
# (c) 2023 Virtual Cable S.L.U.
#
"""
Author: Adolfo Gómez, dkmaster at dkmon dot com
"""
import atexit
import datetime
import ipaddress
import logging
import os
import secrets
import ssl
import tempfile
import typing

from cryptography import x509
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.hazmat.primitives.serialization import BestAvailableEncryption, Encoding, PrivateFormat

from udsactor import consts, types

logger = logging.getLogger(__name__)


def generate_cert(hostnames: typing.Union[list[str], str]) -> types.CertificateInfo:
    """Generates a self signed certificate for the agent"""
    if isinstance(hostnames, str):
        hostnames = [hostnames]
    # Generate our key
    key = rsa.generate_private_key(public_exponent=65537, key_size=2048)

    # Generate the server certificate
    name = x509.Name(
        [
            x509.NameAttribute(x509.NameOID.COMMON_NAME, hostnames[0]),
        ]
    )
    cert = (
        x509.CertificateBuilder()
        .subject_name(name)
        .issuer_name(name)
        .public_key(key.public_key())
        .serial_number(x509.random_serial_number())
        .not_valid_before(datetime.datetime.now() - datetime.timedelta(days=1))
        .not_valid_after(datetime.datetime.now() + datetime.timedelta(days=3650))
        .add_extension(
            x509.BasicConstraints(ca=False, path_length=None),
            critical=True,
        )
        .add_extension(
            x509.SubjectAlternativeName([x509.IPAddress(ipaddress.ip_address(i)) for i in hostnames]),
            critical=False,
        )
        .sign(private_key=key, algorithm=hashes.SHA256())
    )
    password = secrets.token_urlsafe(32)

    # Return the key and the certificate
    return types.CertificateInfo(
        key.private_bytes(
            encoding=Encoding.PEM,
            format=PrivateFormat.TraditionalOpenSSL,
            encryption_algorithm=BestAvailableEncryption(password.encode('utf-8')),
        ).decode('utf-8'),
        cert.public_bytes(Encoding.PEM).decode('utf-8'),
        password,
        ciphers=consts.SECURE_CIPHERS,
    )

