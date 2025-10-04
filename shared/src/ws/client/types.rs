use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "kind", content = "msg")]
pub enum RpcMessage {
    // Requests with id
    LoginRequest(LoginRequest),
    ScreenshotRequest(ScreenshotRequest),
    ScriptExecRequest(ScriptExecRequest),

    // Responses with id
    LoginResponse(LoginResponse),
    ScreenshotResponse(ScreenshotResponse),
    ScriptExecResponse(ScriptExecResponse),

    // Notifications (no id)
    Ping(Ping),
    Pong(Pong),
    LogoffRequest(LogoffRequest),
    LogoutRequest(LogoutRequest),
    ShowMessage(ShowMessage),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub session_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScreenshotRequest;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScreenshotResponse {
    pub result: String,  // base64 encoded image
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LogoffRequest;

/// Payload for logout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequest {
    pub username: String,
    pub session_type: String,
    pub callback_url: String,
    pub session_id: String,
}


#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginResponse {
    pub ip: String,
    pub hostname: String,
    pub deadline: Option<u32>,
    pub max_idle: Option<u32>,
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Ping;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Pong;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShowMessage {
    pub text: String,
}
