use anyhow::Result;

use shared::{
    log,
    ws::{
        server::ServerInfo,
        types::{ScreenshotRequest, ScreenshotResponse},
        wait_for_request, wait_response,
    },
};

use crate::platform;

// Owned ServerInfo and Platform
pub async fn worker(server_info: ServerInfo, _platform: platform::Platform) -> Result<()> {
    // Screenshot request come from broker, goes to wsclient, wait for response and send back to broker
    // for this, we use trackers for request/response matching
    let tracker = server_info.tracker.clone();
    let mut rx = server_info.wsclient_to_workers.subscribe();
    while let Some(env) = wait_for_request::<ScreenshotRequest>(&mut rx, None).await {
        log::debug!("Received ScreenshotRequest");
        let req_id = if let Some(id) = env.id {
            id
        } else {
            log::error!("ScreenshotRequest missing id");
            continue;
        };

        // Register the request
        let (resolver_rx, id) = tracker.register().await;

        // Send screenshot request to wsclient
        let envelope: shared::ws::types::RpcEnvelope<shared::ws::types::RpcMessage> =
            shared::ws::types::RpcEnvelope {
                id: Some(id),
                msg: shared::ws::types::RpcMessage::ScreenshotRequest(ScreenshotRequest),
            };

        if let Err(e) = server_info.workers_to_wsclient.send(envelope).await {
            log::error!("Failed to send ScreenshotRequest to wsclient: {}", e);
            tracker.deregister(id).await;
        } else {
            log::info!("Sent ScreenshotRequest to wsclient with id {}", id);
        }

        // Wait for response
        let response = wait_response::<ScreenshotResponse>(
            resolver_rx,
            None,
            Some(std::time::Duration::from_secs(3)),
        )
        .await;
        if let Ok(screenshot_response) = response {
            // Send response back to broker
            tracker
                .resolve_ok(
                    req_id,
                    shared::ws::types::RpcMessage::ScreenshotResponse(screenshot_response.0),
                )
                .await;
        }
    }
    Ok(())
}
