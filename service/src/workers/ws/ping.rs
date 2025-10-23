use anyhow::Result;

use shared::{
    log,
    ws::{
        server::ServerContext,
        types::{Ping, Pong, RpcEnvelope, RpcMessage},
        wait_message_arrival,
    },
};

use crate::platform;

pub async fn worker(server_info: ServerContext, platform: platform::Platform) -> Result<()> {
    // Note that logout is a simple notification. No response expected (in fact, will return "ok" immediately)
    let mut rx = server_info.from_ws.subscribe();
    while let Some(env) = wait_message_arrival::<Ping>(&mut rx, Some(platform.get_stop())).await {
        log::debug!("Received Ping with id {:?}", env.id);
        // Send back Pong with same payload
        server_info
            .to_ws
            .send(RpcEnvelope {
                id: env.id,
                msg: RpcMessage::Pong(Pong(env.msg.0)),
            })
            .await?;
        log::debug!("Sent Pong response");
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

        let wsclient_to_workers = server_info.from_ws.clone();
        let _handle = tokio::spawn(async move {
            worker(server_info, platform).await.unwrap();
        });

        // Wait to have at least one receiver
        while wsclient_to_workers.receiver_count() == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        log::info!("wsclient_to_workers has receiver");

        // Send 3 ping requests
        for _i in 0..3 {
            let req = RpcEnvelope {
                id: None,
                msg: RpcMessage::Ping(Ping(Vec::new())),
            };
            if let Err(e) = wsclient_to_workers.send(req) {
                log::error!("Failed to send LogoutRequest: {}", e);
            }
        }
        // Wait a bit to let processing happen
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Inspect dummy broker_api
        log::info!("calls: {:?}", calls.dump());
    }
}
