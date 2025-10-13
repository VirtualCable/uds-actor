use anyhow::Result;

use shared::{
    log,
    ws::{server::ServerInfo, types::ScreenshotRequest, wait_for_request},
};

use crate::platform;

// Owned ServerInfo and Platform
pub async fn worker(server_info: ServerInfo, _platform: platform::Platform) -> Result<()> {
    // Screenshot request come from broker, goes to wsclient, wait for response and send back to broker
    // for this, we use trackers for request/response matching
    let tracker = server_info.tracker.clone();
    let mut rx = server_info.wsclient_to_workers.subscribe();
    if let Some(env) = wait_for_request::<ScreenshotRequest>(&mut rx, None).await {
        log::debug!("Received ScreenshotRequest");

        // Register the request
        let (resolver_rx, id) = tracker.register().await;
        
        // Send screenshot request to wsclient
        let envelope: shared::ws::types::RpcEnvelope<shared::ws::types::RpcMessage> = shared::ws::types::RpcEnvelope {
            id: Some(id),
            msg: shared::ws::types::RpcMessage::ScreenshotRequest(ScreenshotRequest),
        };

        if let Err(e) = server_info.workers_to_wsclient.send(envelope).await {
            log::error!("Failed to send ScreenshotRequest to wsclient: {}", e);
            tracker.deregister(id).await;
        } else {
            log::info!("Sent ScreenshotRequest to wsclient with id {}", id);
        }
    }
    Ok(())
}
