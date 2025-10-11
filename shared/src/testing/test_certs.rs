// Only for tests
const CERT_PEM: &[u8] = include_bytes!("../../../testcerts/cert.pem");
const KEY_PEM: &[u8] = include_bytes!("../../../testcerts/key.pem");
const KEY_PEM_WITH_PASS: &[u8] = include_bytes!("../../../testcerts/key_pass.pem");
const KEY_PASSWORD : &str = "test_password";

pub fn test_cert_and_key() -> (&'static [u8], &'static [u8]) {
    (CERT_PEM, KEY_PEM)
}

pub fn test_cert_and_key_with_pass() -> (&'static [u8], &'static [u8], &'static str) {
    (CERT_PEM, KEY_PEM_WITH_PASS, KEY_PASSWORD)
}