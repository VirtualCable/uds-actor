use super::*;
use crate::ws::{
    request_tracker::RequestTracker,
    types::{RpcEnvelope, RpcMessage, ScreenshotResponse},
};
use reqwest::Client;
use tokio::sync::{broadcast, mpsc};

use crate::log;

#[tokio::test]
async fn test_get_screenshot() {
    // Initialize logging for test context
    log::setup_logging("debug", crate::log::LogType::Tests);

    // Initialize TLS (self-signed for tests)
    crate::tls::init_tls(None);

    // Prepare channels
    let (inbound_tx, _inbound_rx) = mpsc::channel(100);
    let (outbound_tx, _) = broadcast::channel(100);
    let tracker = RequestTracker::new();

    // Launch server in background
    let (cert_pem, key_pem) = super::test_certs::test_cert_and_key();
    let port = 32423;

    let server_task = tokio::spawn({
        let inbound_tx = inbound_tx.clone();
        let outbound_tx = outbound_tx.clone();
        let state = tracker.clone();
        async move {
            let result = server(cert_pem, key_pem, port, inbound_tx, outbound_tx, state).await;
            if let Err(e) = result {
                log::error!("Server error: {}", e);
                Err(e)
            } else {
                Ok(())
            }
        }
    });

    // Fake WebSocket client: listens for ScreenshotRequest and resolves it
    tokio::spawn({
        let tracker = tracker.clone();
        let mut rx = outbound_tx.subscribe();
        async move {
            while let Ok(msg) = rx.recv().await {
                if let OutboundMsg::Json(val) = msg {
                    log::debug!("Fake client received: {:?}", val);
                    if let Ok(env) = serde_json::from_value::<RpcEnvelope<RpcMessage>>(val.clone())
                    {
                        log::debug!("Parsed envelope: {:?}", env);
                        if let RpcMessage::ScreenshotRequest(_) = env.msg {
                            log::debug!("Received ScreenshotRequest with id {:?}", env.id);
                            // Build a ScreenshotResponse with the same id
                            let resp = RpcMessage::ScreenshotResponse(ScreenshotResponse {
                                result: "fake_base64_image".into(),
                            });
                            if let Some(id) = env.id {
                                tracker.resolve_ok(id, resp).await;
                            }
                            break;
                        }
                    }
                }
            }
        }
    });

    // Give server a moment to start
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Make HTTP request to the server
    let client = Client::builder()
        .danger_accept_invalid_certs(true) // accept self-signed certs in tests
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap();
    let url = format!("https://localhost:{}/actor/-secret-/screenshot", port);
    let resp = client
        .get(&url)
        .send()
        .await
        .unwrap()
        .json::<ScreenshotResponse>()
        .await
        .unwrap();

    // Validate the response
    assert_eq!(resp.result, "fake_base64_image");

    // Stop server
    server_task.abort();
}
