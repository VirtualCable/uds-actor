use shared::{log, ws::server::ServerInfo};

use crate::platform;

pub mod logoff;

#[allow(dead_code)]
pub async fn create_workers(server_info: ServerInfo, platform: platform::Platform) {
    // Logoff worker
    let _ = tokio::spawn({
        log::info!("Logoff worker created");
        let server_info = server_info.clone();
        let platform = platform.clone();
        async move {
            if let Err(e) = logoff::handle_logoff(server_info, platform).await {
                log::error!("Logoff worker error: {:?}", e);
            }
        }
    }).await;
}
