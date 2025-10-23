use anyhow::Result;

use shared::{
    log,
    ws::{server::ServerContext, types::Close, wait_message_arrival},
};

use crate::platform;

pub async fn worker(server_info: ServerContext, platform: platform::Platform) -> Result<()> {
    // Note that logout is a simple notification. No response expected (in fact, will return "ok" immediately)
    let mut rx = server_info.from_ws.subscribe();
    let broker_api = platform.broker_api();
    while let Some(env) = wait_message_arrival::<Close>(&mut rx, Some(platform.get_stop())).await {
        log::debug!("Received LogoutRequest with id {:?}", env.id);
        let user_info = platform.get_user_info().write().await.take();
        if let Some(user) = user_info {
            let interfaces = platform.operations().get_network_info()?;
            if let Err(err) = broker_api
                .write()
                .await
                .logout(
                    interfaces.as_slice(),
                    (user.username.clone() + " (closed)").as_str(),
                    user.session_type.as_str(),
                    user.session_id.as_deref().unwrap_or(""),
                )
                .await
            {
                log::error!("Logout failed for user {}: {:?}", user.username, err);
            } else {
                log::debug!("Processed LogoutRequest for user {}", user.username);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {

    use shared::ws::types::{RpcEnvelope, RpcMessage};

    use super::*;
    use crate::testing::mock;
    use std::time::Duration;

    #[tokio::test]
    async fn test_logout_worker() {
        log::setup_logging("debug", shared::log::LogType::Tests);
        let server_info = mock::mock_server_info().await;
        let mocked_platform = mock::mock_platform().await;
        let platform = mocked_platform.platform.clone();
        let calls = mocked_platform.calls.clone();
        platform.config().write().await.master_token = Some("mastertoken".into());

        // Set a logged-in user
        platform
            .get_user_info()
            .write()
            .await
            .replace(platform::UserInfo {
                username: "testuser".into(),
                session_type: "testsession".into(),
                session_id: Some("testsessionid".into()),
            });

        let wsclient_to_workers = server_info.from_ws.clone();
        let _handle = tokio::spawn(async move {
            worker(server_info, platform).await.unwrap();
        });

        // Wait to have at least one receiver
        while wsclient_to_workers.receiver_count() == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        log::info!("wsclient_to_workers has receiver");

        // Send 3 close requests
        for _i in 0..3 {
            let req = RpcEnvelope {
                id: None,
                msg: RpcMessage::Close(Close),
            };
            if let Err(e) = wsclient_to_workers.send(req) {
                log::error!("Failed to send LogoutRequest: {}", e);
            }
        }
        // Wait a bit to let processing happen
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Inspect dummy broker_api
        log::info!("calls: {:?}", calls.dump());

        assert!(calls.count_calls("broker_api::logout(") == 1);
    }
}
