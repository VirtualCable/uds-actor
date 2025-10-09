use super::*;
use reqwest::Client;
use tokio::task::JoinHandle;

/// Helper that starts the real server in the background and returns everything needed
async fn spawn_server() -> (
    String,                         // base URL (e.g. "http://127.0.0.1:12345")
    Platform,                       // the manager so we can call stop()
    JoinHandle<anyhow::Result<()>>, // server handle
    Client,                         // cliente HTTP
) {
    shared::log::setup_logging("debug", shared::log::LogType::Tests);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (platform, _calls) = crate::testing::fake::create_platform(None, None, None, None).await;
    let platform_task = platform.clone();
    let handle = tokio::spawn(async move { run_server(listener, platform_task).await });
    let client = Client::new();
    (format!("http://{}", addr), platform, handle, client)
}

#[tokio::test]
async fn test_ping() {
    let (base_url, platform, handle, client) = spawn_server().await;

    let res = client
        .post(format!("{}/ping", base_url))
        .send()
        .await
        .unwrap();
    platform.session_manager().stop().await;

    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "pong");

    assert!(handle.await.unwrap().is_ok());
}

#[tokio::test]
async fn test_logout() {
    let (base_url, platform, handle, client) = spawn_server().await;

    let res = client
        .post(format!("{}/logout", base_url))
        .send()
        .await
        .unwrap();

    platform.session_manager().stop().await;

    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "ok");

    // logout already does stop() → the server ends by itself
    assert!(handle.await.unwrap().is_ok());
}

#[tokio::test]
async fn test_screenshot() {
    let (base_url, platform, handle, client) = spawn_server().await;

    let res = client
        .post(format!("{}/screenshot", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let json: serde_json::Value = res.json().await.unwrap();
    let b64 = json["result"].as_str().unwrap();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .unwrap();
    platform.session_manager().stop().await;
    assert!(handle.await.unwrap().is_ok());

    // PNG header check
    assert!(decoded.starts_with(&[0x89, 0x50, 0x4E, 0x47]));
}

#[tokio::test]
async fn test_script() {
    let (base_url, platform, handle, client) = spawn_server().await;

    let res = client
        .post(format!("{}/script", base_url))
        .json(&serde_json::json!({"script": "echo test"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "ok");

    platform.session_manager().stop().await;
    assert!(handle.await.unwrap().is_ok());
}

#[tokio::test]
async fn test_message() {
    let (base_url, platform, handle, client) = spawn_server().await;

    let res = client
        .post(format!("{}/message", base_url))
        .json(&serde_json::json!({"message": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "ok");

    platform.session_manager().stop().await;
    assert!(handle.await.unwrap().is_ok());
}
