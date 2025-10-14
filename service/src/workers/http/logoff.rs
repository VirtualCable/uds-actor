use anyhow::Result;

use shared::{
    log,
    ws::{server::ServerInfo, types::LogoffRequest, wait_for_request},
};

use crate::platform;

pub async fn worker(server_info: ServerInfo, platform: platform::Platform) -> Result<()> {
    // Note that logoff is a simple notification. No response expected (in fact, will return "ok" immediately)
    let mut rx = server_info.wsclient_to_workers.subscribe();
    if let Some(_env) = wait_for_request::<LogoffRequest>(&mut rx, Some(platform.get_stop())).await {
        log::debug!("Received LogoffRequest");
        // Send logoff to wsclient
        let envelope = shared::ws::types::RpcEnvelope {
            id: None,
            msg: shared::ws::types::RpcMessage::LogoffRequest(LogoffRequest),
        };
        if let Err(e) = server_info.workers_to_wsclient.send(envelope).await {
            log::error!("Failed to send LogoffRequest to wsclient: {}", e);
        } else {
            log::info!("Sent LogoffRequest to wsclient");
        }
    }

    Ok(())
}
