use anyhow::Result;
use axum::http::StatusCode;
use axum::{Extension, Json, Router, extract::Path, routing::get};
use chrono::Utc;

use super::OutboundMsg;
use crate::ws::types::RpcEnvelope;
use crate::{
    log,
    ws::{
        types::{RpcMessage, ScreenshotRequest, ScreenshotResponse},
        wait_response,
    },
};

/// GET /actor/{secret}/screenshot
pub async fn get_screenshot(
    Path(_secret): Path<String>,
    Extension(state): Extension<super::ServerState>,
) -> Result<Json<ScreenshotResponse>, StatusCode> {
    let tracker = state.tracker.clone();
    let outbound_tx = state.outbound_tx.clone();

    // Generate a unique id
    let id = Utc::now().timestamp_millis() as u64;
    log::info!("Screenshot requested via WebSocket API with id {}", id);

    // Register the request
    let rx = tracker.register(id).await;

    // Build the envelope with the typed request
    let envelope = RpcEnvelope {
        id: Some(id),
        msg: RpcMessage::ScreenshotRequest(ScreenshotRequest),
    };

    // Serialize and send
    let val = serde_json::to_value(&envelope).unwrap();
    let _ = outbound_tx.send(OutboundMsg::Json(val));

    wait_response::<ScreenshotResponse>(rx, None).await
}

pub fn routes() -> Router {
    Router::new().route("/actor/{secret}/screenshot", get(get_screenshot))
}
