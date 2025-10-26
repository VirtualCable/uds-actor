use anyhow::Result;

use shared::{
    log,
    ws::{types::Close, wait_message_arrival},
};

use crate::platform;

pub async fn worker(platform: platform::Platform) -> Result<()> {
    let mut rx = platform.ws_client().from_ws.subscribe();
    while let Some(_env) = wait_message_arrival::<Close>(&mut rx, Some(platform.stop())).await {
        log::info!("Received close request, performing close");
        platform.stop().set();
        // TODO: May we logoffo the user or not?
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use shared::ws::types::{Close, RpcEnvelope, RpcMessage};

    use crate::testing::mock::mock_platform;

    use super::*;

    #[tokio::test]
    async fn test_close_worker_stops() {
        shared::log::setup_logging("debug", shared::log::LogType::Tests);
        // Mock platform
        let (platform, _calls, _, _) = mock_platform(None, None, None, None, 43910).await;

        let stop = platform.stop();
        // Run alive worker
        let worker_handle = tokio::spawn(async move {
            let res =
                tokio::time::timeout(std::time::Duration::from_secs(10), super::worker(platform))
                    .await;
            log::info!("Close worker finished with result: {:?}", res);
        });

        // Stop the worker after 1 second
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        stop.set();

        // Wait for the worker to finish
        let _ = worker_handle.await;
    }

    #[tokio::test]
    async fn test_close_worker_closes() {
        shared::log::setup_logging("debug", shared::log::LogType::Tests);
        // Mock platform
        let (platform, _calls, _, _) = mock_platform(None, None, None, None, 43910).await;
        let from_ws = platform.ws_client().from_ws.clone();
        let stop = platform.stop();

        // Run alive worker
        let worker_handle = tokio::spawn(async move {
            let res =
                tokio::time::timeout(std::time::Duration::from_secs(10), super::worker(platform))
                    .await;
            log::info!("Alive worker finished with result: {:?}", res);
        });

        // Wait until from_ws has a subscriber
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        // Any message that is not LogoffRequest will be ignored
        let msg = RpcEnvelope::<RpcMessage> {
            id: None,
            msg: RpcMessage::Ping(shared::ws::types::Ping(b"test".to_vec())),
        };
        from_ws.send(msg).unwrap();
        // Wait a bit to ensure message is processed and ignored
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        // Stop should not be set
        assert!(!stop.is_set());
        assert!(from_ws.is_empty());

        // Send Close request
        let msg = RpcEnvelope::<RpcMessage> {
            id: None,
            msg: RpcMessage::Close(Close),
        };
        from_ws.send(msg).unwrap();

        // Wait for the worker to finish
        let _ = worker_handle.await;
        // End of tests
    }
}
