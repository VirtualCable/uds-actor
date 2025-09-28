use axum::{Extension, Json, Router, routing::post};
use base64::engine::{Engine as _, general_purpose::STANDARD};

#[cfg(test)]
use fake_actions as actions;

#[cfg(not(test))]
use shared::actions;

use crate::platform::Platform;

mod types;

async fn ping() -> &'static str {
    "pong"
}

async fn logout(Extension(state): Extension<types::AppState>) -> &'static str {
    let _ = actions::logoff().await;
    // Notify session manager to stop
    state.platform.session_manager().stop().await;
    "ok"
}

async fn screenshot() -> Json<types::ScreenshotResponse> {
    let data = actions::screenshot().await.unwrap_or_default();
    // Encode to base64 using the standard engine
    let encoded = STANDARD.encode(&data);
    Json(types::ScreenshotResponse { result: encoded })
}

async fn script(Json(req): Json<types::ScriptRequest>) -> &'static str {
    _ = actions::run_script(&req.script).await;
    "ok"
}

async fn message(Json(req): Json<types::MessageRequest>) -> &'static str {
    let _ = actions::show_message(&req.message).await;
    "ok"
}

pub async fn run_server(
    listener: tokio::net::TcpListener,
    platform: Platform,
) -> anyhow::Result<()> {
    // Register with server
    let api = platform.api();
    let callback_url = format!("http://{}", listener.local_addr().unwrap());
    api.write().await.register(&callback_url).await?;

    let app = Router::new()
        .route("/ping", post(ping))
        .route("/logout", post(logout))
        .route("/screenshot", post(screenshot))
        .route("/script", post(script))
        .route("/message", post(message))
        .layer(Extension(types::AppState {
            platform: platform.clone(),
        }));

    let addr = listener.local_addr().unwrap();
    println!("Listening on http://{}", addr);

    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        platform.session_manager().wait().await;
    });

    let res = server
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e));

    // Unregister from server
    let _ = api.write().await.unregister().await;
    res
}

// Test implementations for actions
#[cfg(test)]
mod fake_actions {
    pub async fn logoff() -> Result<(), ()> {
        Ok(())
    }
    pub async fn screenshot() -> Result<Vec<u8>, ()> {
        Ok(vec![0x89, 0x50, 0x4E, 0x47])
    } // PNG header
    pub async fn run_script(_s: &str) -> Result<String, ()> {
        Ok("ok".into())
    }
    pub async fn show_message(_m: &str) -> Result<String, ()> {
        Ok("ok".into())
    }
}

#[cfg(test)]
mod tests;
