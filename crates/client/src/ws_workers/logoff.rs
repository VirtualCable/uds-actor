use anyhow::Result;

use shared::{
    log,
    ws::{types::LogoffRequest, wait_message_arrival},
};

use crate::platform;

// Owned ServerInfo and Platform
pub async fn worker(platform: platform::Platform) -> Result<()> {
    let mut rx = platform.ws_client().from_ws.subscribe();
    while let Some(_env) =
        wait_message_arrival::<LogoffRequest>(&mut rx, Some(platform.stop())).await
    {
        log::info!("Received logoff request, performing logoff");
        platform.stop().set();
        if let Err(e) = platform.system().logoff() {
            log::error!("Forced logoff failed: {e:#}");
            // Record the enforcement failure so main() will not emit a clean
            // session-end claim to the broker for a session still alive.
            crate::LOGOFF_FAILED.store(true, std::sync::atomic::Ordering::SeqCst);
            // Report the failure to the broker log stream (the service's logger
            // worker forwards LogRequest to the broker).
            let _ = platform
                .ws_client()
                .to_ws
                .send(shared::ws::types::RpcEnvelope {
                    id: None,
                    msg: shared::ws::types::RpcMessage::LogRequest(
                        shared::ws::types::LogRequest {
                            level: shared::ws::types::LogLevel::Error,
                            message: format!("Forced logoff failed: {e:#}"),
                        },
                    ),
                })
                .await;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use shared::ws::types::{RpcEnvelope, RpcMessage};

    use crate::testing::mock::mock_platform;

    use super::*;

    #[tokio::test]
    async fn test_logoff_worker_stops() {
        shared::log::setup_logging("debug", shared::log::LogType::Tests);
        // Mock platform
        let (platform, _calls, _, _) = mock_platform(None, None, None, None, 43910).await;

        let stop = platform.stop();
        // Run alive worker
        let worker_handle = tokio::spawn(async move {
            let res =
                tokio::time::timeout(std::time::Duration::from_secs(10), super::worker(platform))
                    .await;
            log::info!("Alive worker finished with result: {:?}", res);
        });

        // Stop the worker after 1 second
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        stop.set();

        // Wait for the worker to finish
        let _ = worker_handle.await;
    }

    #[tokio::test]
    async fn test_logoff_worker_logs_off() {
        shared::log::setup_logging("debug", shared::log::LogType::Tests);
        // Mock platform
        let (platform, calls, _, _) = mock_platform(None, None, None, None, 43910).await;
        let from_ws = platform.ws_client().from_ws.clone();

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
        calls.assert_not_called("operations::logoff()");
        assert!(from_ws.is_empty());

        // Send logoff request
        let msg = RpcEnvelope::<RpcMessage> {
            id: None,
            msg: RpcMessage::LogoffRequest(LogoffRequest {}),
        };
        from_ws.send(msg).unwrap();

        // Wait for the worker to finish
        let _ = worker_handle.await;

        calls.assert_called("operations::logoff()");
        // End of tests
    }
}
