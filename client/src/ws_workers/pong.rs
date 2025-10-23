use anyhow::Result;

use shared::{
    log,
    ws::{types::Pong, wait_message_arrival},
};

use crate::platform;

// Owned ServerInfo and Platform
pub async fn worker(platform: platform::Platform) -> Result<()> {
    let mut rx = platform.ws_client().from_ws.subscribe();
    while let Some(_env) = wait_message_arrival::<Pong>(&mut rx, Some(platform.stop())).await {
        log::info!("Received ping response (pong), ok");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::testing::mock::mock_platform;

    use super::*;

    #[tokio::test]
    async fn test_pong_worker_stops() {
        shared::log::setup_logging("debug", shared::log::LogType::Tests);
        // Mock platform
        let (platform, _calls, _, _) = mock_platform(None, None, None, None, 43910).await;

        let stop = platform.stop();
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

}