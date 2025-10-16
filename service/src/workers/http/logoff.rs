use anyhow::Result;

use shared::{
    log,
    ws::{server::ServerContext, types::LogoffRequest, wait_for_request},
};

use crate::platform;

pub async fn worker(server_info: ServerContext, platform: platform::Platform) -> Result<()> {
    // Note that logoff is a simple notification. No response expected (in fact, will return "ok" immediately)
    let mut rx = server_info.wsclient_to_workers.subscribe();
    while let Some(_env) =
        wait_for_request::<LogoffRequest>(&mut rx, Some(platform.get_stop())).await
    {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::mock;
    use std::{sync::Arc, time::Duration};

    use shared::ws::types::{RpcEnvelope, RpcMessage};
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_logoff_worker() {
        log::setup_logging("debug", shared::log::LogType::Tests);
        let (server_info, mut wsclient_to_workers_rx) =
            mock::mock_server_info_with_worker_rx().await;
        let (platform, calls) = mock::mock_platform().await;
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
                msg: RpcMessage::LogoffRequest(LogoffRequest),
            };
            if let Err(e) = wsclient_to_workers.send(req) {
                log::error!("Failed to send LogoutRequest: {}", e);
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
