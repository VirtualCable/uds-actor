use anyhow::Result;

use shared::{
    log,
    ws::{types::Ping},
};

use crate::platform;

// Owned ServerInfo and Platform
async fn _worker(platform: platform::Platform, timeout: std::time::Duration) -> Result<()> {
    let stop = platform.get_stop();
    // Err means timeout
    while stop
        .wait_timeout(timeout)
        .await
        .is_err()
    {
        // Sending ping
        let ws_client = platform.ws_client();
        ws_client
            .to_ws
            .send(shared::ws::types::RpcEnvelope {
                id: None,
                msg: shared::ws::types::RpcMessage::Ping(Ping(b"ping".to_vec())),
            })
            .await?;
        log::debug!("Sent ping request");
    }
    log::debug!("Alive worker stopping");
    Ok(())
}

pub async fn worker(platform: platform::Platform) -> Result<()> {
    // Use a 30 seconds timeout for pings
    _worker(platform, std::time::Duration::from_secs(30)).await
}


#[cfg(test)]
mod tests {
    use crate::testing::mock::mock_platform;

    use super::*;

    #[tokio::test]
    async fn test_alive_worker_stops() {
        shared::log::setup_logging("debug", shared::log::LogType::Tests);
        // Mock platform
        let (platform, _calls, _, _) = mock_platform(None, None, 43910).await;

        let stop = platform.get_stop();
        // Run alive worker
        let worker_handle = tokio::spawn(async move {
            let res = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                super::worker(platform),
            )
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
    async fn test_alive_worker_sends_ping() {
        shared::log::setup_logging("debug", shared::log::LogType::Tests);
        // Mock platform
        let (platform, _calls, _from_rx_receiver, mut to_ws_receiver) = mock_platform(None, None, 43910).await;

        let stop = platform.get_stop();
        // Run alive worker
        let worker_handle = tokio::spawn(async move {
            let res = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                super::_worker(platform, std::time::Duration::from_secs(1)),
            )
            .await;
            log::info!("Alive worker finished with result: {:?}", res);
        });

        // Wait 3 seconds to allow some pings. This will give us at least 2 pings (1 every second)
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        stop.set();

        // Extract pings from ws_client
        let mut ping_count = 0;
        while let Ok(msg) = to_ws_receiver.try_recv() {
            if let shared::ws::types::RpcMessage::Ping(_) = msg.msg {
                ping_count += 1;
            }
        }
        assert!(ping_count >= 2, "Expected at least 2 pings, got {}", ping_count);

        // Wait for the worker to finish
        let _ = worker_handle.await;
    }

}
