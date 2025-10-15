use anyhow::Result;

use shared::{
    log,
    ws::{server::ServerInfo, types::PreConnect, wait_for_request},
};

use crate::{actions, platform};

// Owned ServerInfo and Platform
pub async fn worker(server_info: ServerInfo, platform: platform::Platform) -> Result<()> {
    // Note that logoff is a simple notification. No response expected (in fact, will return "ok" immediately)
    let mut rx = server_info.wsclient_to_workers.subscribe();
    while let Some(env) = wait_for_request::<PreConnect>(&mut rx, Some(platform.get_stop())).await {
        log::debug!("Received PreConnect: {:?}", env.msg);
        // Process the Preconnect. If protocol is rdp, ensure the user can rdp
        let msg = env.msg;
        if msg.protocol.to_lowercase() == "rdp" {
            if let Err(e) = platform.operations().ensure_user_can_rdp(&msg.user) {
                log::error!("Failed to ensure user can RDP: {}", e);
            } else {
                log::info!("Ensured user can RDP: {}", msg.user);
            }
        }
        // If the a pre command is configured, run it
        actions::process_command(&platform, actions::CommandType::PreConnect).await;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::dummy;
    use std::time::Duration;

    use shared::ws::types::{RpcEnvelope, RpcMessage};

    #[tokio::test]
    async fn test_preconnect_worker() {
        log::setup_logging("debug", shared::log::LogType::Tests);
        let server_info = dummy::create_dummy_server_info().await;
        let (platform, calls) = dummy::create_dummy_platform().await;
        platform.config().write().await.master_token = Some("mastertoken".into());

        let wsclient_to_workers = server_info.wsclient_to_workers.clone();

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
                msg: RpcMessage::PreConnect(PreConnect {
                    user: "testuser".into(),
                    protocol: "rdp".into(),
                    ip: Some("192.168.1.1".into()),
                    hostname: Some("testhost".into()),
                    udsuser: Some("udsuser".into()),
                }),
            };
            if let Err(e) = wsclient_to_workers.send(req) {
                log::error!("Failed to send MessageRequest: {}", e);
            }
        }
        // Wait a bit to let processing happen
        tokio::time::sleep(Duration::from_millis(200)).await;

        // No calls here, only redirects messages to wsclient
        log::info!("calls: {:?}", calls.dump());
        assert!(calls.count_calls("operations::ensure_user_can_rdp(") == 3);
    }
}
