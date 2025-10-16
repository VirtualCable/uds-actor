use anyhow::Result;

use shared::{
    log,
    ws::{
        server::ServerContext,
        types::{UUidRequest, UUidResponse},
        wait_for_request,
    },
};

use crate::platform;

// Owned ServerInfo and Platform
pub async fn worker(server_info: ServerContext, platform: platform::Platform) -> Result<()> {
    // Screenshot request come from broker, goes to wsclient, wait for response and send back to broker
    // for this, we use trackers for request/response matching
    let tracker = server_info.tracker.clone();
    let mut rx = server_info.wsclient_to_workers.subscribe();
    while let Some(env) = wait_for_request::<UUidRequest>(&mut rx, Some(platform.get_stop())).await
    {
        log::debug!("Received UUidRequest");
        let req_id = if let Some(id) = env.id {
            id
        } else {
            log::error!("UUidRequest missing id");
            continue;
        };

        // Unique id is own_token from config
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::mock;
    use std::{time::Duration};

    use shared::ws::types::{RpcEnvelope, RpcMessage};

    #[tokio::test]
    async fn test_uniqueid_worker() {
        log::setup_logging("debug", shared::log::LogType::Tests);
        let server_info = mock::mock_server_info().await;
        let (platform, calls) = mock::mock_platform().await;
        platform.config().write().await.master_token = Some("mastertoken".into());
        platform.config().write().await.own_token = Some("own_token".into());

        let wsclient_to_workers = server_info.wsclient_to_workers.clone();
        let tracker = server_info.tracker.clone();

        let _handle = tokio::spawn(async move {
            worker(server_info, platform).await.unwrap();
        });

        // Wait to have at least one receiver
        while wsclient_to_workers.receiver_count() == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        log::info!("wsclient_to_workers has receiver");

        // Send 3 uniqueid requests
        let mut receivers: Vec<_> = vec![];
        for _i in 0..3 {
            // Register in tracker first
            let (recv, id) = tracker.register().await;
            receivers.push(recv);
            log::info!("Registered request id: {}", id);
            let req = RpcEnvelope {
                id: Some(id),
                msg: RpcMessage::UUidRequest(UUidRequest),
            };
            if let Err(e) = wsclient_to_workers.send(req) {
                log::error!("Failed to send MessageRequest: {}", e);
            }
        }
        // Wait a bit to let processing happen
        tokio::time::sleep(Duration::from_millis(200)).await;

        // No calls here, only redirects messages to wsclient
        log::info!("calls: {:?}", calls.dump());

        // Messages should be in receivers
        for recv in receivers {
            if let Ok(env) = tokio::time::timeout(Duration::from_millis(500), recv).await {
                let msg = env.unwrap();
                log::info!("Received response: {:?}", msg);
                if let RpcMessage::UUidResponse(resp) = msg {
                    assert_eq!(resp.0, "own_token");
                } else {
                    panic!("Unexpected message type");  
                }
            }
        }
    }
}
