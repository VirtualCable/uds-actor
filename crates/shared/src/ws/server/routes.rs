use anyhow::Result;
use axum::http::StatusCode;
use axum::{
    Extension, Json, Router,
    response::Html,
    routing::{get, post},
};
use chrono::Utc;
use serde::Serialize;

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

/// Broker-facing response envelope, matching the 3.x/4.0 actor contract:
/// every public endpoint answers `{"result": ..., "error": ...}` so the
/// broker can always parse the body as JSON and read `result`
/// (see server comms.py `_execute_actor_request`: `r.json()` then
/// `js["result"]`).
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub result: Option<T>,
    pub error: Option<String>,
}

fn ok<T: Serialize>(result: T) -> Json<ApiResponse<T>> {
    Json(ApiResponse {
        result: Some(result),
        error: None,
    })
}

fn err<T>(status: StatusCode, msg: &str) -> (StatusCode, Json<ApiResponse<T>>) {
    (
        status,
        Json(ApiResponse {
            result: None,
            error: Some(msg.to_string()),
        }),
    )
}

/// GET /actor/{secret}/screenshot
pub async fn get_screenshot(
    Extension(state): Extension<super::ServerState>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    let tracker = state.tracker.clone();
    let wsclient_to_workers = state.wsclient_to_workers.clone();

    // Register the request
    let (resolver_rx, id) = tracker.register().await;

    // Build the envelope with the typed request
    let envelope = RpcEnvelope {
        id: Some(id),
        msg: RpcMessage::ScreenshotRequest(ScreenshotRequest),
    };

    // Serialize and broadcast
    if let Err(e) = wsclient_to_workers.send(envelope) {
        log::warn!("Failed to broadcast ScreenshotRequest to workers: {e}");
    }

    // Wait for response, with a timeout of 5 seconds. It's more than enough for a screenshot,
    // And more taking into account that we will communicate with the client using WebSocket that
    // is istantaneous (almost :P)
    wait_response::<ScreenshotResponse>(resolver_rx, None, Some(std::time::Duration::from_secs(5)))
        .await
        .map(|resp| ok(resp.0.result))
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))
}

// GET /actor/{secret}/uuid
pub async fn get_uuid(
    Extension(state): Extension<super::ServerState>,
) -> Result<Json<ApiResponse<String>>, (StatusCode, Json<ApiResponse<String>>)> {
    let tracker = state.tracker.clone();
    let wsclient_to_workers = state.wsclient_to_workers.clone();

    // Generate a unique id
    let id = Utc::now().timestamp_millis() as u64;
    log::debug!("UUID requested via WebSocket API with id {}", id);

    // Register the request
    let (resolver_rx, id) = tracker.register().await;

    // Build the envelope with the typed request
    let envelope = RpcEnvelope {
        id: Some(id),
        msg: RpcMessage::UUidRequest(UUidRequest),
    };

    // Serialize and broadcast
    if let Err(e) = wsclient_to_workers.send(envelope) {
        log::warn!("Failed to broadcast UUidRequest to workers: {e}");
    }

    // Wait for response
    // Timeout of 2 seconds should much much much more than enough :)
    wait_response::<UUidResponse>(resolver_rx, None, Some(std::time::Duration::from_secs(2)))
        .await
        .map(|uuid| ok(uuid.0.0.clone()))
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))
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
) -> Json<ApiResponse<&'static str>> {
    log::info!("Logout requested via WebSocket API");
    let envelope = RpcEnvelope {
        id: None,
        msg: RpcMessage::LogoffRequest(LogoffRequest),
    };

    if let Err(e) = state.wsclient_to_workers.send(envelope) {
        log::warn!("Failed to broadcast LogoffRequest to workers: {e}");
    }

    ok("ok")
}

pub async fn post_message(
    Extension(state): Extension<super::ServerState>,
    Json(req): Json<MessageRequest>,
) -> Json<ApiResponse<&'static str>> {
    log::info!("Message display requested via WebSocket API");
    let envelope = RpcEnvelope {
        id: None,
        msg: RpcMessage::MessageRequest(req),
    };

    if let Err(e) = state.wsclient_to_workers.send(envelope) {
        log::warn!("Failed to broadcast MessageRequest to workers: {e}");
    }

    ok("ok")
}

pub async fn post_script(
    Extension(state): Extension<super::ServerState>,
    Json(req): Json<ScriptExecRequest>,
) -> Json<ApiResponse<&'static str>> {
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

    ok("ok")
}

// Note: the broker may send extra fields (udsuser_uuid, userservice_uuid,
// service_type); serde ignores unknown fields, so both the 4.0 and the 5.0
// payloads deserialize into PreConnect.
pub async fn post_pre_connect(
    Extension(state): Extension<super::ServerState>,
    Json(req): Json<PreConnect>,
) -> Json<ApiResponse<&'static str>> {
    log::info!("Pre-connect requested via WebSocket API");
    let envelope = RpcEnvelope {
        id: None,
        msg: RpcMessage::PreConnect(PreConnect {
            user: req.user,
            protocol: req.protocol,
            ip: req.ip,
            hostname: req.hostname,
            udsuser: req.udsuser,
        }),
    };

    if let Err(e) = state.wsclient_to_workers.send(envelope) {
        log::warn!("Failed to broadcast PreConnect to workers: {e}");
    }

    ok("ok")
}

pub fn routes() -> Router {
    Router::new()
        .route("/actor/{secret}/screenshot", get(get_screenshot))
        .route("/actor/{secret}/uuid", get(get_uuid))
        .route("/actor/{secret}/logout", post(post_logout))
        .route("/actor/{secret}/message", post(post_message))
        .route("/actor/{secret}/script", post(post_script))
        // The 3.x/4.0 broker calls this "preConnect" (camelCase) and axum
        // routes are case-sensitive, so register both spellings.
        .route("/actor/{secret}/preconnect", post(post_pre_connect))
        .route("/actor/{secret}/preConnect", post(post_pre_connect))
        .route("/", get(get_information))
}
