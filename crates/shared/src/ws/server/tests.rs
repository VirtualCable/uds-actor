use anyhow::Result;
use tokio::sync::{broadcast, mpsc};

use tokio_tungstenite::{Connector, connect_async_tls_with_config, tungstenite::Message};

use super::*;
use crate::ws::{request_tracker::RequestTracker, types::RpcMessage};

use crate::log;

type ServerTaskResult = (
    ServerStartInfo,
    tokio::task::JoinHandle<Result<(), Box<dyn std::error::Error + Send + Sync>>>,
    broadcast::Receiver<RpcEnvelope<RpcMessage>>,
    mpsc::Sender<RpcEnvelope<RpcMessage>>, // workers→client: caller must hold it or Task B ends and closes the socket
);

fn create_test_server_task(port: u16, secret: &str) -> ServerTaskResult {
    log::setup_logging("debug", crate::log::LogType::Tests);
    crate::tls::init_tls(None);

    // Create the single channel for workers → WS client
    let (workers_tx, workers_rx) = mpsc::channel::<RpcEnvelope<RpcMessage>>(100);

    // Broadcast channel for WS client → workers (receiver is returned so tests
    // can assert what reaches the workers)
    let (wsclient_to_workers, wsclient_rx) = broadcast::channel::<RpcEnvelope<RpcMessage>>(100);

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
        ciphers: None,
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
        wsclient_rx,
        workers_tx,
    )
}

#[tokio::test]
async fn test_server_starts_and_stops() {
    let (server_info, server_task, _, _workers_tx) = create_test_server_task(32500, "-secret-");

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
    let (server_info, server_task, _, _workers_tx) = create_test_server_task(32433, "-secret-");

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

#[tokio::test]
async fn test_ws_second_client_rejected() {
    let (server_info, server_task, _, _workers_tx) = create_test_server_task(32435, "-secret-");

    // Wait a moment for the server to start
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let url = format!("wss://localhost:{}/ws", server_info.port);
    let connector = Connector::Rustls(crate::tls::noverify::client_config());

    // First client: claims the single slot.
    let (_ws1, _resp) = connect_async_tls_with_config(url.clone(), None, true, Some(connector.clone()))
        .await
        .expect("first WebSocket handshake failed");

    // Second client while the first is connected: rejected (409 -> handshake error).
    assert!(
        connect_async_tls_with_config(url, None, true, Some(connector))
            .await
            .is_err(),
        "a second client must not be able to claim the ws slot"
    );

    server_info.stop.set();
    tokio::time::timeout(tokio::time::Duration::from_secs(5), server_task)
        .await
        .expect("Server did not stop in time")
        .expect("Server task panicked")
        .expect("Server returned an error");
}

#[tokio::test]
async fn test_ws_peer_flood_drops_connection() {
    let (server_info, server_task, _, _workers_tx) = create_test_server_task(32436, "-secret-");

    // Wait a moment for the server to start
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let url = format!("wss://localhost:{}/ws", server_info.port);
    let connector = Connector::Rustls(crate::tls::noverify::client_config());
    let (mut ws_stream, _resp) = connect_async_tls_with_config(url, None, true, Some(connector))
        .await
        .expect("WebSocket handshake failed");

    // 40 small frames back to back: exhausts the 32-frame burst budget. The
    // server must drop the connection instead of forwarding the flood (a
    // flood can otherwise make busy workers skip broker directives).
    for _ in 0..40 {
        let _ = ws_stream
            .send(Message::Text(
                r#"{"id":null,"kind":"Ping","msg":[1,2,3]}"#.into(),
            ))
            .await;
    }

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        futures_util::StreamExt::next(&mut ws_stream),
    )
    .await;
    match result {
        // Stream ended or errored (connection dropped): expected
        Ok(None) | Ok(Some(Err(_))) | Ok(Some(Ok(Message::Close(_)))) => {}
        other => panic!("flooded peer should be dropped, got {other:?}"),
    }

    server_info.stop.set();
    tokio::time::timeout(tokio::time::Duration::from_secs(5), server_task)
        .await
        .expect("Server did not stop in time")
        .expect("Server task panicked")
        .expect("Server returned an error");
}

#[tokio::test]
async fn test_ws_rejects_broker_directives_from_client() {
    let (server_info, server_task, mut workers_rx, _workers_tx) =
        create_test_server_task(32434, "-secret-");

    // Wait a moment for the server to start
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let url = format!("wss://localhost:{}/ws", server_info.port);
    let connector = Connector::Rustls(crate::tls::noverify::client_config());
    let (mut ws_stream, _resp) = connect_async_tls_with_config(url, None, true, Some(connector))
        .await
        .expect("WebSocket handshake failed");

    // Broker-only directive from a local peer: must NOT reach the workers
    ws_stream
        .send(Message::Text(
            r#"{"id":null,"kind":"PreConnect","msg":{"user":"attacker","protocol":"rdp","ip":null,"hostname":null,"udsuser":null}}"#.into(),
        ))
        .await
        .expect("Failed to send PreConnect");

    // Client-originated message: must reach the workers broadcast
    ws_stream
        .send(Message::Text(
            r#"{"id":null,"kind":"LogRequest","msg":{"level":50000,"message":"from-client"}}"#.into(),
        ))
        .await
        .expect("Failed to send LogRequest");

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let mut saw_log_request = false;
    while let Ok(env) = workers_rx.try_recv() {
        match env.msg {
            RpcMessage::LogRequest(_) => saw_log_request = true,
            other => panic!("Broker directive reached workers broadcast: {:?}", other),
        }
    }
    assert!(saw_log_request, "client LogRequest did not reach workers");

    server_info.stop.set();
    // Wait with timeout to avoid hanging tests
    tokio::time::timeout(tokio::time::Duration::from_secs(5), server_task)
        .await
        .expect("Server did not stop in time")
        .expect("Server task panicked")
        .expect("Server returned an error");
}
