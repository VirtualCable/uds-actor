use std::sync::{Arc, atomic::AtomicU16};

use anyhow::Result;

use futures_util::sink::SinkExt;
use local_ip_address::{local_ip, local_ipv6};
use tokio_tungstenite::{Connector, connect_async_tls_with_config, tungstenite::Message};

use reqwest::Client;
use shared::{
    log, sync::OnceSignal, testing::test_certs, ws::{
        server::{start_server, ServerInfo},
        types::{
            LogoffRequest, MessageRequest, Ping, PreConnect, RpcEnvelope, RpcMessage,
            ScreenshotRequest, ScreenshotResponse, ScriptExecRequest, UUidRequest, UUidResponse,
        },
        wait_for_request, wait_response,
    }
};

// Port counter to avoid collisions
static NEXT_PORT: AtomicU16 = AtomicU16::new(32420);

async fn create_test_server_task(secret: &str) -> (ServerInfo, tokio::task::JoinHandle<()>, u16) {
    let port = NEXT_PORT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    log::setup_logging("debug", crate::log::LogType::Tests);
    shared::tls::init_tls(None);

    let stop = Arc::new(OnceSignal::new());
    let cert_info = test_certs::test_certinfo();

    let (server_info, handle) = start_server(cert_info, stop.clone(), secret.into(), Some(port))
        .await
        .unwrap();
    // Wait a moment for the server to start
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    (server_info, handle, port)
}

async fn get_request(url: &str) -> Result<String> {
    let client = Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap();
    let resp = client.get(url).send().await.unwrap();
    let status = resp.status();
    let body = resp.text().await.unwrap();

    assert!(status.is_success(), "Error (status {status}):\n{body}");
    Ok(body)
}

async fn post_request<U: serde::Serialize>(url: &str, json: &U) -> Result<String> {
    let client = Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(true)
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .unwrap();
    let resp = client.post(url).json(json).send().await.unwrap();
    let status = resp.status();
    let body = resp.text().await.unwrap();

    if !status.is_success() {
        return Err(anyhow::anyhow!("Error (status {status}):\n{body}"));
    }

    Ok(body)
}

#[tokio::test]
async fn test_get_screenshot() {
    let (server_info, server_task, port) = create_test_server_task("-secret-").await;

    let tracker = server_info.tracker.clone();
    let wsclient_to_workers = server_info.wsclient_to_workers.clone();

    // Fake WebSocket client that responds to ScreenshotRequest
    tokio::spawn({
        let tracker = tracker.clone();
        let mut rx = wsclient_to_workers.subscribe();
        async move {
            if let Some(env) = wait_for_request::<ScreenshotRequest>(&mut rx, None).await {
                log::debug!("Received ScreenshotRequest with id {:?}", env.id);
                if let Some(id) = env.id {
                    tracker
                        .resolve_ok(
                            id,
                            RpcMessage::ScreenshotResponse(ScreenshotResponse {
                                result: "fake_base64_image".into(),
                            }),
                        )
                        .await;
                }
            }
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let body = get_request(&format!(
        "https://localhost:{}/actor/-secret-/screenshot",
        port
    ))
    .await
    .unwrap();

    let result: ScreenshotResponse = serde_json::from_str::<ScreenshotResponse>(&body)
        .unwrap_or_else(|_| panic!("Error on response:\n{body}"));

    assert_eq!(result.result, "fake_base64_image");

    server_task.abort();
}

#[tokio::test]
async fn test_get_uuid() {
    let (server_info, server_task, port) = create_test_server_task("-secret-").await;

    let tracker = server_info.tracker.clone();
    let wsclient_to_workers = server_info.wsclient_to_workers.clone();
    // Fake WebSocket client that responds to UUidRequest
    tokio::spawn({
        let tracker = tracker.clone();
        let mut rx = wsclient_to_workers.subscribe();
        async move {
            if let Some(env) = wait_for_request::<UUidRequest>(&mut rx, None).await {
                log::debug!("Received UUidRequest with id {:?}", env.id);
                if let Some(id) = env.id {
                    tracker
                        .resolve_ok(
                            id,
                            RpcMessage::UUidResponse(UUidResponse("fake-uuid-1234".into())),
                        )
                        .await;
                }
            }
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let result = get_request(&format!("https://localhost:{}/actor/-secret-/uuid", port,))
        .await
        .unwrap();

    assert_eq!(result, "fake-uuid-1234");

    server_task.abort();
}

#[tokio::test]
async fn test_information() {
    let (_server_info, server_task, port) = create_test_server_task("-secret-").await;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let result = get_request(&format!("https://localhost:{}/", port))
        .await
        .unwrap();

    assert!(result.contains("UDS Actor"));

    server_task.abort();
}

#[tokio::test]
async fn test_post_logout() {
    let (server_info, server_task, port) = create_test_server_task("-secret-").await;

    // Subscribe to receive the LogoffRequest
    let mut rx = server_info.wsclient_to_workers.subscribe();

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let result = post_request(
        &format!("https://localhost:{}/actor/-secret-/logout", port),
        &(),
    )
    .await
    .unwrap();
    assert_eq!(result, "ok");

    // Execute in a timeout to avoid hanging forever
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        wait_for_request::<LogoffRequest>(&mut rx, None).await;
    })
    .await
    .unwrap(); // Fail if timeout

    server_task.abort();
}

#[tokio::test]
pub async fn test_post_message() {
    let (server_info, server_task, port) = create_test_server_task("-secret-").await;

    // Subscribe to receive the MessageRequest
    let mut rx = server_info.wsclient_to_workers.subscribe();

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let result = post_request(
        &format!("https://localhost:{}/actor/-secret-/message", port),
        &MessageRequest {
            message: "test message".into(),
        },
    )
    .await
    .unwrap();
    assert_eq!(result, "ok");

    // Execute in a timeout to avoid hanging forever
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let res = wait_for_request::<MessageRequest>(&mut rx, None)
            .await
            .unwrap();
        assert_eq!(res.msg.message, "test message");
    })
    .await
    .unwrap(); // Fail if timeout

    server_task.abort();
}

#[tokio::test]
pub async fn test_post_script() {
    let (server_info, server_task, port) = create_test_server_task("-secret-").await;

    // Subscribe to receive the ScriptExecRequest
    let mut rx = server_info.wsclient_to_workers.subscribe();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let result = post_request(
        &format!("https://localhost:{}/actor/-secret-/script", port),
        &ScriptExecRequest {
            script_type: "script_type".into(),
            script: "test script".into(),
        },
    )
    .await
    .unwrap();

    assert_eq!(result, "ok");

    // Execute in a timeout to avoid hanging forever
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let res = wait_for_request::<ScriptExecRequest>(&mut rx, None)
            .await
            .unwrap();
        assert_eq!(res.msg.script, "test script");
        assert_eq!(res.msg.script_type, "script_type");
    })
    .await
    .unwrap(); // Fail if timeout

    server_task.abort();
}

#[tokio::test]
pub async fn test_post_pre_connect() {
    let (server_info, server_task, port) = create_test_server_task("-secret-").await;
    // Subscribe to receive the PreConnect
    let mut rx = server_info.wsclient_to_workers.subscribe();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let result = post_request(
        &format!("https://localhost:{}/actor/-secret-/preconnect", port),
        &(),
    )
    .await
    .unwrap();

    assert_eq!(result, "ok");
    // Execute in a timeout to avoid hanging forever
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        let res = wait_for_request::<PreConnect>(&mut rx, None).await;
        assert!(res.is_some());
    })
    .await
    .unwrap(); // Fail if timeout

    server_task.abort();
}

#[tokio::test]
async fn test_secret_invalid() {
    let (_server_info, server_task, port) = create_test_server_task("-secret-").await;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let resp = reqwest::Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
        .get(format!(
            "https://localhost:{}/actor/wrong-secret/screenshot",
            port
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), reqwest::StatusCode::FORBIDDEN);

    server_task.abort();
}

#[tokio::test]
#[ignore = "Requires network access"]
async fn test_ws_no_localhost_ipv4() {
    let (_server_info, server_task, port) = create_test_server_task("-secret-").await;
    let local_ip = local_ip().unwrap();
    log::debug!("Local IP address: {}:{}", local_ip, port);

    // Wait a moment for the server to start
    // tokio::time::sleep(std::time::Duration::from_millis(15000)).await;

    let resp = reqwest::Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
        .get(format!("https://{}:{}/ws", local_ip, port))
        .send()
        .await;
    log::debug!("Response: {:?}", resp);
    let resp = resp.unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::NOT_FOUND);
    server_task.abort();
}

#[tokio::test]
#[ignore = "Requires network access"]
async fn test_ws_no_localhost_ipv6() {
    let (_server_info, server_task, port) = create_test_server_task("-secret-").await;
    let local_ip = local_ipv6().unwrap();
    log::debug!("Local IP address: {}", local_ip);

    // Wait a moment for the server to start
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let resp = reqwest::Client::builder()
        .use_rustls_tls()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap()
        .get(format!("https://[{}]:{}/ws", local_ip, port))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::NOT_FOUND);
    server_task.abort();
}

// Ensure ws works
#[tokio::test]
#[ignore = "Requires network access"]
async fn test_ws_connect_insecure_tls() {
    let (server_info, server_task, port) = create_test_server_task("-secret-").await;

    let mut rx = server_info.wsclient_to_workers.subscribe();

    // Wait a moment for the server to start
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Build the WebSocket URL (TLS enabled, but self-signed)
    let url = format!("wss://localhost:{}/ws", port);

    // Create a connector that disables certificate verification
    let connector = Connector::Rustls(shared::tls::noverify::client_config());

    // Perform the WebSocket handshake with custom TLS config
    let (mut ws_stream, _resp) = connect_async_tls_with_config(
        url,
        None, // no additional request headers
        true, // allow insecure
        Some(connector),
    )
    .await
    .expect("WebSocket handshake failed");

    // Send a test message
    ws_stream
        .send(Message::Ping("ping".into()))
        .await
        .expect("Failed to send message");

    // do not have response, but sends on tx a ping message

    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        // let res = rx.recv().await;
        wait_for_request::<Ping>(&mut rx, None).await;
    })
    .await
    .unwrap(); // Fail if timeout

    server_task.abort();
}

#[tokio::test]
#[ignore = "Requires network access"]
async fn test_ws_msg_with_envelope_id() {
    let (server_info, server_task, port) = create_test_server_task("-secret-").await;
    
    // Wait a moment for the server to start
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Build the WebSocket URL (TLS enabled, but self-signed)
    let url = format!("wss://localhost:{}/ws", port);

    // Create a connector that disables certificate verification
    let connector = Connector::Rustls(shared::tls::noverify::client_config());

    // Perform the WebSocket handshake with custom TLS config
    let (mut ws_stream, _resp) = connect_async_tls_with_config(
        url,
        None, // no additional request headers
        true, // allow insecure
        Some(connector),
    )
    .await
    .expect("WebSocket handshake failed");

    let tracker = server_info.tracker.clone();
    // Register the request
    let (resolver_rx, id) = tracker.register().await;

    let message = Message::Text(
        serde_json::to_string(&RpcEnvelope {
            id: Some(id),
            msg: RpcMessage::Ping(Ping("ping".into())),
        })
        .unwrap()
        .into(),
    );

    // Send a test message
    ws_stream
        .send(message)
        .await
        .expect("Failed to send message");

    // do not have response, but sends on tx a ping message
    let response =
        wait_response::<Ping>(resolver_rx, None, Some(std::time::Duration::from_secs(5))).await;

    log::debug!("Response: {:?}", response);
    assert!(response.is_ok());

    server_task.abort();
}
