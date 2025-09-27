use crate::session::SessionManagement;

#[derive(Clone)]
pub struct AppState {
    pub session_manager: std::sync::Arc<dyn SessionManagement + Send + Sync>,
}

#[derive(serde::Deserialize)]
pub struct ScriptRequest {
    pub script: String,
}

#[derive(serde::Deserialize)]
pub struct MessageRequest {
    pub message: String,
}

#[derive(serde::Serialize)]
pub struct ScreenshotResponse {
    pub result: String,  // Base64 encoded image
}