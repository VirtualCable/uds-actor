use shared::{log, ws::server::ServerInfo};

use crate::platform;

pub mod logging;
pub mod login;
pub mod logout;

#[allow(dead_code)]
async fn create_workers(server_info: ServerInfo, platform: platform::Platform) {
    // Login worker
    let _ = tokio::spawn({
        log::info!("Log worker created");
        let server_info = server_info.clone();
        let platform = platform.clone();
        async move {
            if let Err(e) = logging::handle_log(server_info, platform).await {
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
            if let Err(e) = login::handle_login(server_info, platform).await {
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
            if let Err(e) = logout::handle_logout(server_info, platform).await {
                log::error!("Logout worker error: {:?}", e);    
            }
        }
    }).await;
}
