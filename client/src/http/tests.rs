use super::*;
use reqwest::Client;
use shared::sync::event::{Event, EventLike};
use std::sync::Arc;
use tokio::task::JoinHandle;

#[derive(Clone)]
struct DummySessionManager {
    event: Event,
}

impl DummySessionManager {
    fn new() -> Self {
        Self {
            event: Event::new(),
        }
    }
}

#[async_trait::async_trait]
impl crate::session::SessionManagement for DummySessionManager {
    async fn wait(&self) {
        let ev = self.event.clone();
        tokio::task::spawn_blocking(move || ev.wait())
            .await
            .unwrap();
    }

    async fn is_running(&self) -> bool {
        !self.event.is_set()
    }

    async fn stop(&self) {
        self.event.signal();
    }

    async fn wait_timeout(&self, timeout: std::time::Duration) -> bool {
        let ev = self.event.clone();
        tokio::task::spawn_blocking(move || ev.wait_timeout(timeout))
            .await
            .unwrap()
    }
}

/// Helper que arranca el servidor real en background y devuelve todo lo necesario
async fn spawn_server() -> (
    String,                         // base URL (ej. "http://127.0.0.1:12345")
    Platform,                       // el manager para poder hacer stop()
    JoinHandle<anyhow::Result<()>>, // handle del servidor
    Client,                         // cliente HTTP
) {
    let manager = Arc::new(DummySessionManager::new());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let platform = crate::platform::Platform::new_with_params(Some(manager.clone()), None, None, None);
    let plaform_task = platform.clone();
    let handle = tokio::spawn(async move { run_server(listener, plaform_task).await });

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
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "pong");

    platform.session_manager().stop().await;
    assert!(handle.await.unwrap().is_ok());
}

#[tokio::test]
async fn test_logout() {
    let (base_url, _platform, handle, client) = spawn_server().await;

    let res = client
        .post(format!("{}/logout", base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "ok");

    // logout ya hace stop() → el servidor termina solo
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
    // PNG header check
    assert!(decoded.starts_with(&[0x89, 0x50, 0x4E, 0x47]));

    platform.session_manager().stop().await;
    assert!(handle.await.unwrap().is_ok());
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
