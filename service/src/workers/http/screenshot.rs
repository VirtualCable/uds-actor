use anyhow::Result;

use shared::{
    log,
    ws::{
        server::ServerInfo,
        types::{UUidRequest, UUidResponse},
        wait_for_request,
    },
};

use crate::platform;

// Owned ServerInfo and Platform
pub async fn worker(server_info: ServerInfo, platform: platform::Platform) -> Result<()> {
    // Screenshot request come from broker, goes to wsclient, wait for response and send back to broker
    // for this, we use trackers for request/response matching
    let tracker = server_info.tracker.clone();
    let mut rx = server_info.wsclient_to_workers.subscribe();
    while let Some(env) = wait_for_request::<UUidRequest>(&mut rx, None).await {
        log::debug!("Received UUidRequest");
        let req_id = if let Some(id) = env.id {
            id
        } else {
            log::error!("UUidRequest missing id");
            continue;
        };

        let uuid = platform
            .config()
            .read()
            .await
            .own_token
            .clone()
            .unwrap_or_default();
        let response = UUidResponse(uuid);

        // Send response back to broker
        tracker
            .resolve_ok(
                req_id,
                shared::ws::types::RpcMessage::UUidResponse(response),
            )
            .await;
    }
    Ok(())
}
