use serde::{Deserialize, Serialize};

// Shared types for WebSocket messages
// But reexport here for consistency
pub use crate::broker::api::types::{LoginResponse, LogLevel};

pub type RequestId = u64;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RpcEnvelope<T> {
    pub id: Option<RequestId>,
    #[serde(flatten)]
    pub msg: T,
}


#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RpcError {
    pub code: u32,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(tag = "kind", content = "msg")]
pub enum RpcMessage {
    // Requests with id
    LoginRequest(LoginRequest),
    ScreenshotRequest(ScreenshotRequest),
    ScriptExecRequest(ScriptExecRequest),
    UUidRequest(UUidRequest), // No payload

    // Responses with id
    LoginResponse(LoginResponse),
    ScreenshotResponse(ScreenshotResponse),
    ScriptExecResponse(ScriptExecResponse),
    // Message does not have a response
    UUidResponse(UUidResponse), // UUID as string

    // Notifications (no id)
    Ping(Ping),                // Used to maintain connection alive
    Pong(Pong),                // Response to Ping, same payload
    LogoffRequest(LogoffRequest), // From broker for client
    PreConnect(PreConnect),       // From broker for server
    LogoutRequest(LogoutRequest), // From client ws for the broker
    LogRequest(LogRequest),       // From client ws for the broker
    MessageRequest(MessageRequest),
    Close(Close),                  // From client ws to server

    // Error response with
    Error(RpcError),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub session_type: String,
}

// Login response is same as broker API response

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScreenshotRequest;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScreenshotResponse {
    pub result: String, // base64 encoded image
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScriptExecRequest {
    #[serde(rename = "type")]
    pub script_type: String,
    pub script: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScriptExecResponse {
    pub result: String,
}

/// Payload for logout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequest {
    pub username: String,
    pub session_type: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRequest {
    pub level: LogLevel, // Log level
    pub message: String, // Log message
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageRequest {
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UUidRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UUidResponse(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreConnect {
    pub user: String,
    pub protocol: String,
    pub ip: Option<String>,
    pub hostname: Option<String>,
    pub udsuser: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoffRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ping(pub Vec<u8>); // Payload is arbitrary data, to be sent back as-is

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pong(pub Vec<u8>); // Payload is arbitrary data received from Ping

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Close;