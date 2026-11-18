use rustls::crypto::CryptoProvider;

use crate::log;

pub mod certool;
pub mod ciphers;
pub mod noverify;

// NOTE: Debug is implemented manually (not derived) so the private key and its
// password can never reach any log sink through a {:?} render (error paths in
// certool.rs format this type).
#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub struct CertificateInfo {
    pub key: String,
    pub certificate: String,
    pub password: Option<String>,
    pub ciphers: Option<String>,
}

impl std::fmt::Debug for CertificateInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CertificateInfo")
            .field("key", &format_args!("[REDACTED: {} bytes]", self.key.len()))
            .field("certificate", &self.certificate)
            .field(
                "password",
                &self.password.as_ref().map(|_| "[REDACTED]".to_string()),
            )
            .field("ciphers", &self.ciphers)
            .finish()
    }
}

// Ensure only one initialization happens
static INIT: std::sync::Once = std::sync::Once::new();

pub fn init_tls(ciphers_list: Option<&str>) {
    INIT.call_once(|| {
        // Build a provider with your custom cipher list
        log::debug!("Initializing TLS with ciphers: {:?}", ciphers_list);
        let provider: CryptoProvider = ciphers::provider(ciphers_list); // Defaults to all ciphers if None
        // Install it as the global default
        provider
            .install_default()
            .expect("failed to install default provider");
    });
}
