use tokio::sync::{broadcast, mpsc};

use super::*;
use crate::ws::{
    request_tracker::RequestTracker,
    types::{RpcMessage, ScreenshotResponse, ScreenshotRequest},
    wait_for_request,
};
use reqwest::Client;

use crate::log;

#[tokio::test]
async fn test_get_screenshot() {
    log::setup_logging("debug", crate::log::LogType::Tests);
    crate::tls::init_tls(None);

    let (inbound_tx, _inbound_rx) = mpsc::channel(100);
    let (outbound_tx, _) = broadcast::channel(100);
    let tracker = RequestTracker::new();

    let (cert_pem, key_pem) = super::test_certs::test_cert_and_key();
    let port = 32423;
    let notify = Arc::new(tokio::sync::Notify::new());

    let server_task = tokio::spawn({
        let inbound_tx = inbound_tx.clone();
        let outbound_tx = outbound_tx.clone();
        let state = tracker.clone();
        let stop = notify.clone();
        async move {
            let result = server(cert_pem, key_pem, port, inbound_tx, outbound_tx, state, stop).await;
            if let Err(e) = result {
                log::error!("Server error: {}", e);
                Err(e)
            } else {
                Ok(())
            }
        }
    });

    // Fake WebSocket client usando el helper
    tokio::spawn({
        let tracker = tracker.clone();
        let rx = outbound_tx.subscribe();
        async move {
            if let Some(env) = wait_for_request::<ScreenshotRequest>(rx, None).await {
                log::debug!("Received ScreenshotRequest with id {:?}", env.id);
                if let Some(id) = env.id {
                    tracker.resolve_ok(
                        id,
                        RpcMessage::ScreenshotResponse(ScreenshotResponse {
                            result: "fake_base64_image".into(),
                        }),
                    ).await;
                }
            }
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
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

    assert_eq!(resp.result, "fake_base64_image");

    server_task.abort();
}
