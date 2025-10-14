use anyhow::Result;

use shared::{
    log,
    ws::{server::ServerInfo, types::LogoutRequest, wait_for_request},
};

use crate::platform;

pub async fn worker(server_info: ServerInfo, platform: platform::Platform) -> Result<()> {
    // Note that logout is a simple notification. No response expected (in fact, will return "ok" immediately)
    let mut rx = server_info.wsclient_to_workers.subscribe();
    while let Some(env) = wait_for_request::<LogoutRequest>(&mut rx, Some(platform.get_stop())).await {
        log::debug!("Received LogoutRequest with id {:?}", env.id);
        let broker_api = platform.broker_api();
        // Clone api to avoid holding the lock during await
        let interfaces = platform.operations().get_network_info()?;
        if let Err(err) = broker_api
            .write()
            .await
            .logout(
                interfaces.as_slice(),
                env.msg.username.as_str(),
                env.msg.session_type.as_str(),
                env.msg.session_id.as_str(),
            )
            .await
        {
            log::error!("Logout failed for user {}: {:?}", env.msg.username, err);
        } else {
            log::debug!("Processed LogoutRequest for user {}", env.msg.username);
        }
    }

    Ok(())
}


#[cfg(test)]
mod tests {

    use shared::ws::types::{RpcEnvelope, RpcMessage};

    use super::*;
    use crate::testing::dummy;
    use std::time::Duration;

    #[tokio::test]
    async fn test_logout_worker() {
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

        // Send 3 logout requests
        for i in 0..3 {
            let req = RpcEnvelope {
                id: None,
                msg: RpcMessage::LogoutRequest(LogoutRequest {
                    username: format!("user{}", i),
                    session_type: "test".into(),
                    session_id: format!("session{}", i),
                }),
            };
            if let Err(e) = wsclient_to_workers.send(req) {
                log::error!("Failed to send LogoutRequest: {}", e);
            }
        }
        // Wait a bit to let processing happen
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Inspect dummy broker_api
        log::info!("calls: {:?}", calls.dump());

        assert!(calls.count_calls("broker_api::logout(") == 3);
    }
}
