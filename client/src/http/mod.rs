use axum::{Extension, Json, Router, routing::post};
use base64::engine::{Engine as _, general_purpose::STANDARD};

use crate::platform::Platform;

use shared::log;

mod types;

async fn ping() -> &'static str {
    log::debug!("Ping received via HTTP API");
    "pong"
}

async fn logout(Extension(state): Extension<types::AppState>) -> &'static str {
    log::info!("Logout requested via HTTP API");

    // Even in the case that we have been notified of a logout, we need to ensure the API is called
    // right now. As soon as we implement the websocket version, all of this will be obsolete.
    let _ = state
        .platform
        .api()
        .write()
        .await
        .logout(Some("requested by service"))
        .await;
    // Note: Logoff should initiate a logoff by the OS
    _ = state.platform.operations().logoff();

    // Notify session manager to stop: Note, using logoff should be enough, will close our app
    // and that will stop the session manager
    // state.platform.session_manager().stop().await;

    // Notify server we are logging off
    "ok"
}

async fn screenshot(
    Extension(state): Extension<types::AppState>,
) -> Json<types::ScreenshotResponse> {
    log::info!("Screenshot requested via HTTP API");

    let data = tokio::task::spawn_blocking({
        let platform = state.platform;
        move || platform.operations().get_screenshot().unwrap_or_default()
    })
    .await
    .unwrap_or_default();

    // Encode to base64 using the standard engine
    let encoded = STANDARD.encode(&data);
    Json(types::ScreenshotResponse { result: encoded })
}

async fn script(
    Extension(_state): Extension<types::AppState>,
    Json(_req): Json<types::ScriptRequest>,
) -> &'static str {
    log::info!("Script execution requested via HTTP API");
    let _script = _req.script;
    // TODO: Implement script execution
    "ok"
}

async fn message(
    Extension(state): Extension<types::AppState>,
    Json(req): Json<types::MessageRequest>,
) -> &'static str {
    log::info!("Message display requested via HTTP API");
    let _ = state.platform.notify_user(&req.message).await;
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
        log::error!("Failed to register with server: {}", err);
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
    log::debug!("Listening on http://{}", addr);

    log::debug!("Starting server");
    let server = axum::serve(listener, app).with_graceful_shutdown(async move {
        log::debug!("Waiting for session manager shutdown...");
        platform.session_manager().wait().await;
        log::debug!("Session manager has shut down, stopping server...");
    });

    let res = server
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e));

    log::debug!("Server has shut down: {:?}", res);

    // Unregister from server
    let _ = api.write().await.unregister().await;

    log::debug!("Unregistered from server");
    res
}

#[cfg(test)]
mod tests;
