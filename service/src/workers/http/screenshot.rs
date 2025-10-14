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
pub async fn worker(server_info: ServerInfo, platform: platform::Platform) -> Result<()> {
    // Screenshot request come from broker, goes to wsclient, wait for response and send back to broker
    // for this, we use trackers for request/response matching
    let tracker = server_info.tracker.clone();
    let mut rx = server_info.wsclient_to_workers.subscribe();
    while let Some(env) =
        wait_for_request::<ScreenshotRequest>(&mut rx, Some(platform.get_stop())).await
    {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::dummy;
    use std::{sync::Arc, time::Duration};

    use shared::ws::types::{RpcEnvelope, RpcMessage};
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_screenshot_worker() {
        log::setup_logging("debug", shared::log::LogType::Tests);
        let (server_info, mut wsclient_to_workers_rx) =
            dummy::create_dummy_server_info_with_worker_rx().await;
        let (platform, calls) = dummy::create_dummy_platform().await;
        platform.config().write().await.master_token = Some("mastertoken".into());

        let wsclient_to_workers = server_info.wsclient_to_workers.clone();

        let msg: Arc<RwLock<Vec<RpcEnvelope<RpcMessage>>>> = Arc::new(RwLock::new(Vec::new()));
        // Subscribe to workers_to_wsclient to verify messages sent
        let _handle = tokio::spawn({
            let msg = msg.clone();
            async move {
                loop {
                    let recv_msg = wsclient_to_workers_rx.recv().await.unwrap();
                    log::info!("Received message from workers_to_wsclient: {:?}", recv_msg);
                    msg.write().await.push(recv_msg);
                }
            }
        });

        let _handle = tokio::spawn(async move {
            worker(server_info, platform).await.unwrap();
        });

        // Wait to have at least one receiver
        while wsclient_to_workers.receiver_count() == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        log::info!("wsclient_to_workers has receiver");

        // Send 3 logoff requests
        for _i in 0..3 {
            let req = RpcEnvelope {
                id: None,
                msg: RpcMessage::ScreenshotRequest(ScreenshotRequest),
            };
            if let Err(e) = wsclient_to_workers.send(req) {
                log::error!("Failed to send MessageRequest: {}", e);
            }
        }
        // Wait a bit to let processing happen
        tokio::time::sleep(Duration::from_millis(200)).await;

        // No calls here, only redirects messages to wsclient
        log::info!("calls: {:?}", calls.dump());
        let logged_msgs = msg.read().await;
        log::info!("logged_msgs: {:?}", logged_msgs);
        assert!(logged_msgs.len() == 3);
    }
}
