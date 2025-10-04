use tokio::sync::broadcast;
use serde::Deserialize;
use anyhow::Result;

use super::{Frame, request_state::RequestState};

/// Envelope for all RPC messages coming from the server.
/// - `id`: optional request identifier (present in responses).
/// - `kind`: discriminant to know which variant to deserialize.
/// - `msg`: actual payload, deserialized into the enum below.
#[derive(Debug, Deserialize)]
pub struct RpcEnvelope {
    pub id: Option<u64>,
    pub kind: String,
    pub msg: serde_json::Value,
}

/// Strongly typed RPC messages.
/// Each variant corresponds to a possible `kind`.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", content = "msg")]
pub enum RpcMessage {
    LoginResponse(LoginResponse),
    PingResponse(PingResponse),
    // Add more variants as needed
}

#[derive(Debug, Clone, Deserialize)]
pub struct LoginResponse {
    pub ip: String,
    pub hostname: String,
    pub deadline: Option<u32>,  // Stamp, in seconds, when the session will expire
    pub max_idle: Option<u32>,  // Max idle time in seconds
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PingResponse(pub String);

/// Spawn a resolver task that listens on the broadcast channel,
/// parses incoming frames, and resolves pending requests.
pub fn spawn_resolver(state: RequestState, mut rx: broadcast::Receiver<Frame>) {
    tokio::spawn(async move {
        while let Ok(frame) = rx.recv().await {
            // First, try to parse the envelope
            let parsed: Result<RpcEnvelope, _> = serde_json::from_str(&frame.raw);
            let Ok(env) = parsed else {
                // Not a valid envelope → ignore
                continue;
            };

            // Try to deserialize into a typed RpcMessage
            let parsed_msg: Result<RpcMessage, _> = serde_json::from_value(
                serde_json::json!({
                    "kind": env.kind,
                    "msg": env.msg
                })
            );

            match parsed_msg {
                Ok(msg) => {
                    if let Some(id) = env.id {
                        // Resolve the request with the raw JSON string or typed payload
                        // For now, we just return the raw JSON string to the caller
                        let _ = state.resolve_ok(id, frame.raw.clone()).await;
                    } else {
                        // Notification/event without id → handle separately if needed
                        match msg {
                            RpcMessage::LoginResponse(resp) => {
                                println!("Received LoginResponse (event): {:?}", resp);
                            }
                            RpcMessage::PingResponse(resp) => {
                                println!("Received PingResponse (event): {:?}", resp);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse RpcMessage: {e}");
                }
            }
        }
    });
}
