use shared::{ws::server::ServerContext};

use crate::platform;

// Macros
mod macros;

// Workers for WebSocket handling
mod ws;
// Workers for http handling
mod http;

#[allow(dead_code)]
pub async fn create_workers(server_info: ServerContext, platform: platform::Platform) {
    ws::create_workers(server_info.clone(), platform.clone()).await;
    http::create_workers(server_info, platform).await;
}
