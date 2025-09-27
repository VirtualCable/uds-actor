use axum::{Extension, Json, Router, routing::post};
use base64::engine::{Engine as _, general_purpose::STANDARD};
use shared::actions;

use crate::session::SessionManagement;

mod types;

async fn ping() -> &'static str {
    "pong"
}

async fn logout(Extension(state): Extension<types::AppState>) -> &'static str {
    let _ = actions::logoff().await;
    // Notify session manager to stop
    state.session_manager.stop().await;
    "ok"
}

async fn screenshot() -> Json<types::ScreenshotResponse> {
    let data = actions::screenshot().await.unwrap_or_default();
    // Encode to base64 using the standard engine
    let encoded = STANDARD.encode(&data);
    Json(types::ScreenshotResponse { result: encoded })
}

async fn script(Json(req): Json<types::ScriptRequest>) -> &'static str {
    _ = actions::run_script(&req.script);
    "ok"
}

async fn message(Json(req): Json<types::MessageRequest>) -> &'static str {
    let _ = actions::show_message(&req.message).await;
    "ok"
}

pub async fn run_server(
    listener: tokio::net::TcpListener,
    manager: std::sync::Arc<dyn SessionManagement + Send + Sync>,
) {
    let app = Router::new()
        .route("/ping", post(ping))
        .route("/logout", post(logout))
        .route("/screenshot", post(screenshot))
        .route("/script", post(script))
        .route("/message", post(message))
        .layer(Extension(types::AppState {
            session_manager: manager.clone(),
        }));

    let addr = listener.local_addr().unwrap();
    println!("Listening on http://{}", addr);

    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        manager.wait().await;
    });

    if let Err(e) = server.await {
        eprintln!("server error: {e}");
    }
}

#[cfg(test)]
mod tests;
