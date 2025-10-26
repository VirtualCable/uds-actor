use crate::tls::CertificateInfo;

// Only for tests
const CERT_PEM: &[u8] = include_bytes!("../../../../assets/testcerts/cert.pem");
const KEY_PEM: &[u8] = include_bytes!("../../../../assets/testcerts/key.pem");
const KEY_PEM_WITH_PASS: &[u8] = include_bytes!("../../../../assets/testcerts/key_pass.pem");
const KEY_PASSWORD: &str = "test_password";
const TESTING_CIPHERS: &str = "TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256";

// const TESTING_CIPHERS: &str = "TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256:\
//      ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-RSA-CHACHA20-POLY1305-SHA256";

pub fn test_certinfo() -> CertificateInfo {
    CertificateInfo {
        key: String::from_utf8(KEY_PEM.to_vec()).unwrap(),
        certificate: String::from_utf8(CERT_PEM.to_vec()).unwrap(),
        password: None,
        ciphers: Some(TESTING_CIPHERS.to_string()),
    }
}

pub fn test_certinfo_with_pass() -> CertificateInfo {
    CertificateInfo {
        key: String::from_utf8(KEY_PEM_WITH_PASS.to_vec()).unwrap(),
        certificate: String::from_utf8(CERT_PEM.to_vec()).unwrap(),
        password: Some(KEY_PASSWORD.to_string()),
        ciphers: Some(TESTING_CIPHERS.to_string()),
    }
}
