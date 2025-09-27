use super::*;
use shared::sync::event::{Event, EventLike};
use std::sync::Arc;

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
        tokio::task::spawn_blocking(move || {
            ev.wait();
        })
        .await
        .unwrap();
    }

    async fn is_running(&self) -> bool {
        !self.event.is_set()
    }

    async fn stop(&self) {
        self.event.signal();
    }
}

#[tokio::test]
async fn test_ping() {
    let manager = Arc::new(DummySessionManager::new());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_manager = manager.clone();
    let handle = tokio::spawn(async move {
        run_server(listener, server_manager).await;
    });

    // Petición real
    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://{}/ping", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "pong");

    // Apaga el servidor
    manager.stop().await;
    handle.await.unwrap();
}

#[tokio::test]
async fn test_logout() {
    let manager = Arc::new(DummySessionManager::new());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_manager = manager.clone();
    let handle = tokio::spawn(async move {
        run_server(listener, server_manager).await;
    });

    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://{}/logout", addr))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "ok");

    // Server should stop after logout
    handle.await.unwrap();
}

#[tokio::test]
async fn test_screenshot() {
    let manager = Arc::new(DummySessionManager::new());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_manager = manager.clone();
    let handle = tokio::spawn(async move {
        run_server(listener, server_manager).await;
    });

    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://{}/screenshot", addr))
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

    manager.stop().await;
    handle.await.unwrap();
}

#[tokio::test]
async fn test_script() {
    let manager = Arc::new(DummySessionManager::new());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_manager = manager.clone();
    let handle = tokio::spawn(async move {
        run_server(listener, server_manager).await;
    });

    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://{}/script", addr))
        .json(&serde_json::json!({"script": "echo test"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "ok");

    manager.stop().await;
    handle.await.unwrap();
}

#[tokio::test]
async fn test_message() {
    let manager = Arc::new(DummySessionManager::new());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_manager = manager.clone();
    let handle = tokio::spawn(async move {
        run_server(listener, server_manager).await;
    });

    let client = reqwest::Client::new();
    let res = client
        .post(format!("http://{}/message", addr))
        .json(&serde_json::json!({"message": "hello"}))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "ok");

    manager.stop().await;
    handle.await.unwrap();
}
