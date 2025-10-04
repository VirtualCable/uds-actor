// Only for tests
const CERT_PEM: &[u8] = include_bytes!("../../../../testcerts/cert.pem");
const KEY_PEM: &[u8] = include_bytes!("../../../../testcerts/key.pem");

pub fn test_cert_and_key() -> (&'static [u8], &'static [u8]) {
    (CERT_PEM, KEY_PEM)
}