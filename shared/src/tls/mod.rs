use rustls::crypto::CryptoProvider;

pub mod ciphers;
pub mod noverify;
pub mod certool;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct CertificateInfo {
    pub key: String,
    pub certificate: String,
    pub password: Option<String>,
    pub ciphers: Option<String>,
}


// Ensure only one initialization happens
static INIT: std::sync::Once = std::sync::Once::new();

pub fn init_tls(ciphers_list: Option<&str>) {
    INIT.call_once(|| {
        // Build a provider with your custom cipher list
        let provider: CryptoProvider = ciphers::provider(ciphers_list); // Defaults to all ciphers if None
        // Install it as the global default
        provider
            .install_default()
            .expect("failed to install default provider");
    });
}
