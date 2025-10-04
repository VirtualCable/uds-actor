use std::sync::Arc;
use std::net::TcpStream;
use rustls::{ClientConfig, ClientConnection, RootCertStore, Stream};
use rustls_native_certs::load_native_certs;

use shared::{tls::ciphers, log};

// This test tries to connect to www.example.com:443 using the filtered cipher list
#[test]
#[ignore] // Ignored because it requires network access
fn test_tls_handshake_with_example_com() {
    shared::log::setup_logging("debug", shared::log::LogType::Tests);
    // Pick some ciphers (you can try restricting to one to see if it still works)
    let ciphers = "TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384";

    // Build provider with your filter
    let provider = ciphers::provider(ciphers);

        let certs = load_native_certs().certs;

        let mut root_store = RootCertStore::empty();
        root_store.add_parsable_certificates(certs);

    // Build client config with our cipher suites
    let config = ClientConfig::builder_with_provider(Arc::new(provider))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let server_name = "www.example.com".try_into().unwrap();
    let mut conn = ClientConnection::new(Arc::new(config), server_name).unwrap();

    let mut sock = TcpStream::connect("www.example.com:443").unwrap();
    let mut tls = Stream::new(&mut conn, &mut sock);

    // Try to write an HTTP request and flush
    use std::io::Write;
    write!(tls, "GET / HTTP/1.0\r\nHost: www.example.com\r\n\r\n").unwrap();
    tls.flush().unwrap();

    // Read some response
    use std::io::Read;
    let mut buf = [0u8; 1024];
    let n = tls.read(&mut buf).unwrap();
    assert!(n > 0, "no data received from server");

    let resp = String::from_utf8_lossy(&buf[..n]);
    log::info!("Received response:\n{}", resp);

}
