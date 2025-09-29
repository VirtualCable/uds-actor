use axum::{Extension, Json, Router, routing::post};
use base64::engine::{Engine as _, general_purpose::STANDARD};

use crate::platform::Platform;

mod types;

async fn ping() -> &'static str {
    "pong"
}

async fn logout(Extension(state): Extension<types::AppState>) -> &'static str {
    let _ = state.platform.actions().logoff().await;
    // Notify session manager to stop
    state.platform.session_manager().stop().await;
    _ = state.platform.operations().logoff();
    "ok"
}

async fn screenshot(
    Extension(state): Extension<types::AppState>,
) -> Json<types::ScreenshotResponse> {
    let data = state
        .platform
        .actions()
        .screenshot()
        .await
        .unwrap_or_default();
    // Encode to base64 using the standard engine
    let encoded = STANDARD.encode(&data);
    Json(types::ScreenshotResponse { result: encoded })
}

async fn script(
    Extension(state): Extension<types::AppState>,
    Json(req): Json<types::ScriptRequest>,
) -> &'static str {
    _ = state.platform.actions().run_script(&req.script).await;
    "ok"
}

async fn message(
    Extension(state): Extension<types::AppState>,
    Json(req): Json<types::MessageRequest>,
) -> &'static str {
    let _ = state.platform.actions().show_message(&req.message).await;
    "ok"
}

pub async fn callback_url(listener: &tokio::net::TcpListener) -> String {
    format!("http://{}", listener.local_addr().unwrap())
}

pub async fn run_server(
    listener: tokio::net::TcpListener,
    platform: Platform,
) -> anyhow::Result<()> {
    // Register with server
    let api = platform.api();
    let callback_url = callback_url(&listener).await;

    // If cannot register, set stop and return
    if let Err(err) = api.write().await.register(&callback_url).await {
        shared::log::error!("Failed to register with server: {}", err);
        platform.session_manager().stop().await;
        return Err(anyhow::anyhow!("Failed to register with server: {}", err));
    }

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

#[cfg(test)]
mod tests;
