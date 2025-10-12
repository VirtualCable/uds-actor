use shared::{log, ws::server::ServerInfo};

use crate::platform;

// Workers for WebSocket handling
mod ws;
// Workers for http handling
mod http;

#[allow(dead_code)]
pub async fn create_workers(server_info: ServerInfo, platform: platform::Platform) {
    // Login worker
    let _ = tokio::spawn({
        log::info!("Log worker created");
        let server_info = server_info.clone();
        let platform = platform.clone();
        async move {
            if let Err(e) = ws::logger::handle_log(server_info, platform).await {
                log::error!("Log worker error: {:?}", e);
            }
        }
    }).await;
    // Login worker
    let _ = tokio::spawn({
        log::info!("Login worker created");
        let server_info = server_info.clone();
        let platform = platform.clone();
        async move {
            if let Err(e) = ws::login::handle_login(server_info, platform).await {
                log::error!("Login worker error: {:?}", e);
            }
        }
    }).await;
    // Logout worker
    let _ = tokio::spawn({
        log::info!("Logout worker created");
        let server_info = server_info.clone();
        let platform = platform.clone();
        async move {
            if let Err(e) = ws::logout::handle_logout(server_info, platform).await {
                log::error!("Logout worker error: {:?}", e);    
            }
        }
    }).await;
}
