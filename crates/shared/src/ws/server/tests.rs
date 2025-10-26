use anyhow::Result;
use tokio::sync::{broadcast, mpsc};

use tokio_tungstenite::{Connector, connect_async_tls_with_config, tungstenite::Message};

use super::*;
use crate::ws::{request_tracker::RequestTracker, types::RpcMessage};

use crate::log;

type ServerTaskResult = (
    ServerStartInfo,
    tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
);

fn create_test_server_task(port: u16, secret: &str) -> ServerTaskResult {
    log::setup_logging("debug", crate::log::LogType::Tests);
    crate::tls::init_tls(None);

    // Create the single channel for workers → WS client
    let (_, workers_rx) = mpsc::channel::<RpcEnvelope<RpcMessage>>(100);

    // Broadcast channel for WS client → workers
    let (wsclient_to_workers, _) = broadcast::channel::<RpcEnvelope<RpcMessage>>(100);

    let tracker = RequestTracker::new();
    let cert_info = crate::testing::test_certs::test_certinfo_with_pass();
    let stop = OnceSignal::new();

    let server_info = ServerStartInfo {
        cert_info,
        port,
        workers_to_wsclient: Arc::new(tokio::sync::Mutex::new(workers_rx)), // unique receiver
        wsclient_to_workers: wsclient_to_workers.clone(),
        tracker: tracker.clone(),
        stop: stop.clone(),
        secret: secret.into(),
    };

    let server_info_task = server_info.clone();
    (
        server_info,
        tokio::spawn({
            async move {
                server(&server_info_task).await.map_err(|e| {
                    log::error!("Server error: {}", e);
                    Box::<dyn std::error::Error + Send + Sync>::from(e)
                })
            }
        }),
    )
}

#[tokio::test]
async fn test_server_starts_and_stops() {
    let (server_info, server_task) = create_test_server_task(32500, "-secret-");

    // Wait a bit for server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Stop the server
    server_info.stop.set();

    // Wait with timeout to avoid hanging tests
    tokio::time::timeout(tokio::time::Duration::from_secs(5), server_task)
        .await
        .expect("Server did not stop in time")
        .expect("Server task panicked")
        .expect("Server returned an error");
}

#[tokio::test]
async fn test_server_stops_on_ws_client_connected() {
    let (server_info, server_task) = create_test_server_task(32433, "-secret-");

    // Wait a moment for the server to start
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Build the WebSocket URL (TLS enabled, but self-signed)
    let url = format!("wss://localhost:{}/ws", server_info.port);

    // Create a connector that disables certificate verification
    let connector = Connector::Rustls(crate::tls::noverify::client_config());

    // Perform the WebSocket handshake with custom TLS config
    let (mut ws_stream, _resp) = connect_async_tls_with_config(
        url,
        None, // no additional request headers
        true, // allow insecure
        Some(connector),
    )
    .await
    .expect("WebSocket handshake failed");

    // Send a test message, we don not need a response, just testing it stops
    ws_stream
        .send(Message::Ping("ping".into()))
        .await
        .expect("Failed to send message");

    // Wait a moment to ensure the server processes the message
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    server_info.stop.set();
    // Wait with timeout to avoid hanging tests
    tokio::time::timeout(tokio::time::Duration::from_secs(5), server_task)
        .await
        .expect("Server did not stop in time")
        .expect("Server task panicked")
        .expect("Server returned an error");
}
