use anyhow::Result;
use axum::http::StatusCode;
use axum::{
    Extension, Json, Router,
    response::Html,
    routing::{get, post},
};
use chrono::Utc;

use crate::ws::types::{LogoffRequest, PreConnect, RpcEnvelope};
use crate::{
    log,
    ws::{
        types::{
            MessageRequest, RpcMessage, ScreenshotRequest, ScreenshotResponse, ScriptExecRequest,
            UUidRequest, UUidResponse,
        },
        wait_response,
    },
};

/// GET /actor/{secret}/screenshot
pub async fn get_screenshot(
    Extension(state): Extension<super::ServerState>,
) -> Result<Json<ScreenshotResponse>, StatusCode> {
    let tracker = state.tracker.clone();
    let wsclient_to_workers = state.wsclient_to_workers.clone();

    // Generate a unique id
    let id = Utc::now().timestamp_millis() as u64;
    log::debug!("Screenshot requested via WebSocket API with id {}", id);

    // Register the request
    let resolver_rx = tracker.register(id).await;

        // Build the envelope with the typed request
    let envelope = RpcEnvelope {
        id: Some(id),
        msg: RpcMessage::ScreenshotRequest(ScreenshotRequest),
    };

    // Serialize and broadcast
    if let Err(e) = wsclient_to_workers.send(envelope) {
        log::warn!("Failed to broadcast ScreenshotRequest to workers: {e}");
    }

    wait_response::<ScreenshotResponse>(resolver_rx, None).await
}

// GET /actor/{secret}/uuid
pub async fn get_uuid(
    Extension(state): Extension<super::ServerState>,
) -> Result<String, StatusCode> {
    let tracker = state.tracker.clone();
    let wsclient_to_workers = state.wsclient_to_workers.clone();

    // Generate a unique id
    let id = Utc::now().timestamp_millis() as u64;
    log::debug!("UUID requested via WebSocket API with id {}", id);

    // Register the request
    let resolver_rx = tracker.register(id).await;

    // Build the envelope with the typed request
    let envelope = RpcEnvelope {
        id: Some(id),
        msg: RpcMessage::UUidRequest(UUidRequest),
    };

    // Serialize and broadcast
    if let Err(e) = wsclient_to_workers.send(envelope) {
        log::warn!("Failed to broadcast UUidRequest to workers: {e}");
    }

    // Wait for response, and convert to String
    let val = wait_response::<UUidResponse>(resolver_rx, None).await;
    if let Ok(uuid) = &val {
        Ok(uuid.0.0.clone())
    } else {
        Err(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub async fn get_information() -> Result<Html<String>, StatusCode> {
    Ok(Html(format!(
        "<h1>UDS Actor {}.{}</h1>",
        crate::consts::VERSION,
        crate::consts::BUILD
    )))
}

pub async fn post_logout(
    Extension(state): Extension<super::ServerState>,
) -> Result<&'static str, StatusCode> {
    log::info!("Logout requested via WebSocket API");
    let envelope = RpcEnvelope {
        id: None,
        msg: RpcMessage::LogoffRequest(LogoffRequest),
    };

    if let Err(e) = state.wsclient_to_workers.send(envelope) {
        log::warn!("Failed to broadcast LogoffRequest to workers: {e}");
    }

    Ok("ok")
}

pub async fn post_message(
    Extension(state): Extension<super::ServerState>,
    Json(req): Json<MessageRequest>,
) -> Result<&'static str, StatusCode> {
    log::info!("Message display requested via WebSocket API");
    let envelope = RpcEnvelope {
        id: None,
        msg: RpcMessage::MessageRequest(req),
    };

    if let Err(e) = state.wsclient_to_workers.send(envelope) {
        log::warn!("Failed to broadcast MessageRequest to workers: {e}");
    }

    Ok("ok")
}

pub async fn post_script(
    Extension(state): Extension<super::ServerState>,
    Json(req): Json<ScriptExecRequest>,
) -> Result<&'static str, StatusCode> {
    log::info!("Script execution requested via WebSocket API");
    let envelope = RpcEnvelope {
        id: None,
        msg: RpcMessage::ScriptExecRequest(ScriptExecRequest {
            script_type: req.script_type,
            script: req.script,
        }),
    };

    if let Err(e) = state.wsclient_to_workers.send(envelope) {
        log::warn!("Failed to broadcast ScriptExecRequest to workers: {e}");
    }

    Ok("ok")
}

pub async fn post_pre_connect(
    Extension(state): Extension<super::ServerState>,
) -> Result<&'static str, StatusCode> {
    log::info!("Pre-connect requested via WebSocket API");
    let envelope = RpcEnvelope {
        id: None,
        msg: RpcMessage::PreConnect(PreConnect),
    };

    if let Err(e) = state.wsclient_to_workers.send(envelope) {
        log::warn!("Failed to broadcast PreConnect to workers: {e}");
    }

    Ok("ok")
}

pub fn routes() -> Router {
    Router::new()
        .route("/actor/{secret}/screenshot", get(get_screenshot))
        .route("/actor/{secret}/uuid", get(get_uuid))
        .route("/actor/{secret}/logout", post(post_logout))
        .route("/actor/{secret}/message", post(post_message))
        .route("/actor/{secret}/script", post(post_script))
        .route("/actor/{secret}/preconnect", post(post_pre_connect))
        .route("/", get(get_information))
}
