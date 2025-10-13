use anyhow::Result;

use shared::{
    log,
    ws::{server::ServerInfo, types::MessageRequest, wait_for_request},
};

use crate::platform;

// Owned ServerInfo and Platform
pub async fn worker(server_info: ServerInfo, _platform: platform::Platform) -> Result<()> {
    // Note that logoff is a simple notification. No response expected (in fact, will return "ok" immediately)
    let mut rx = server_info.wsclient_to_workers.subscribe();
    if let Some(env) = wait_for_request::<MessageRequest>(&mut rx, None).await {
        log::debug!("Received MessageRequest");
        // Send logoff to wsclient
        let envelope = shared::ws::types::RpcEnvelope {
            id: None,
            msg: shared::ws::types::RpcMessage::MessageRequest(MessageRequest { message: env.msg.message }),
        };
        if let Err(e) = server_info.workers_to_wsclient.send(envelope).await {
            log::error!("Failed to send MessageRequest to wsclient: {}", e);
        } else {
            log::info!("Sent MessageRequest to wsclient");
        }
    }
    Ok(())
}
