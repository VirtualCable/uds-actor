use anyhow::Result;
use axum_server::tls_rustls::RustlsConfig;
use rustls::version::{TLS12, TLS13};
use std::sync::Arc;

use crate::log;

use rustls::ServerConfig;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};

use pkcs8::EncryptedPrivateKeyInfo;
use pkcs8::der::Decode;
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys}; // necesario para from_der()

use super::{CertificateInfo, ciphers};

pub fn rustls_config_from_pem(cert_info: CertificateInfo) -> Result<RustlsConfig> {
    // Infor for error logging
    let err_cert_info = cert_info.clone();

    // Extract certificate chain
    let cert_pem: Vec<u8> = cert_info.certificate.into();
    let mut cert_reader = cert_pem.as_slice();
    let cert_chain: Vec<CertificateDer<'static>> =
        certs(&mut cert_reader).collect::<Result<Vec<_>, _>>()?;

    let key_pem: Vec<u8> = cert_info.key.into();
    // Extract private key, (possibly decrypting it first if password is Some)
    let private_key: PrivateKeyDer<'static> = match cert_info.password {
        Some(ref pass) if !pass.is_empty() => {
            let pem_block = pem::parse(key_pem.as_slice())?;
            let epki = EncryptedPrivateKeyInfo::from_der(pem_block.contents())
                .map_err(|e| anyhow::anyhow!("Failed to parse encrypted private key: {:?}", e))?;
            let doc = epki.decrypt(pass).map_err(|e| {
                log::info!("Invalid TLS certificate info: {:?}", err_cert_info);
                anyhow::anyhow!(
                    "Failed to decrypt private key with provided password: {:?}",
                    e
                )
            })?;
            // doc.as_bytes() is PKCS#8 DER; convert to PrivatePkcs8KeyDer
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(doc.as_bytes().to_vec()))
        }
        _ => parse_unencrypted_key(key_pem.as_slice())?,
    };

    let provider = ciphers::provider(cert_info.ciphers.as_deref());

    let config: ServerConfig = ServerConfig::builder_with_provider(Arc::new(provider))
        .with_protocol_versions(&[&TLS13, &TLS12])?
        .with_no_client_auth()
        .with_single_cert(cert_chain, private_key)?;

    Ok(RustlsConfig::from_config(Arc::new(config)))
}

fn parse_unencrypted_key(key_pem: &[u8]) -> Result<PrivateKeyDer<'static>> {
    // PKCS#8 unencrypted
    let mut reader = key_pem;
    if let Some(k) = pkcs8_private_keys(&mut reader).next().transpose()? {
        return Ok(PrivateKeyDer::Pkcs8(k));
    }

    // RSA PKCS#1
    let mut reader = key_pem;
    if let Some(k) = rsa_private_keys(&mut reader).next().transpose()? {
        return Ok(PrivateKeyDer::Pkcs1(k));
    }

    Err(anyhow::anyhow!("No valid private key found"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::test_certs;
    use crate::tls::init_tls;

    #[test]
    fn load_unencrypted_key() {
        init_tls(None);

        let cert_info = test_certs::test_certinfo();

        let cfg = rustls_config_from_pem(cert_info);
        assert!(cfg.is_ok(), "should load unencrypted key");
    }

    #[test]
    fn load_encrypted_key_with_password() {
        init_tls(None);

        let cert_info = test_certs::test_certinfo_with_pass();

        let cfg = rustls_config_from_pem(cert_info);
        assert!(cfg.is_ok(), "should load encrypted key with password");
    }

    #[test]
    fn fail_with_wrong_password() {
        init_tls(None);

        let mut cert_info = test_certs::test_certinfo_with_pass();
        cert_info.password = Some("wrong_password".into());

        let cfg = rustls_config_from_pem(cert_info);
        assert!(cfg.is_err(), "should fail with wrong password");
    }
}
