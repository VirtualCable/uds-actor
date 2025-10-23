use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Notify;

use crate::platform;
use crate::testing::mock::mock_platform;

use shared::log;

pub struct TestSetup {
    pub platform: platform::Platform,
    pub calls: shared::testing::mock::Calls,
    pub handle: Option<tokio::task::JoinHandle<()>>,
    pub notify: Arc<Notify>,
}

impl TestSetup {
    pub async fn new<F, Fut>(runner: F) -> Self
    where
        F: FnOnce(platform::Platform) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        log::setup_logging("debug", shared::log::LogType::Tests);
        let mocked_platform = mock_platform().await;
        let platform = mocked_platform.platform.clone();
        let calls = mocked_platform.calls.clone();
        let notify = Arc::new(Notify::new());

        // Run the managed run function in a separate task
        let handle = tokio::spawn({
            let platform = platform.clone();
            let notify = notify.clone();
            async move {
                notify.notified().await; // Wait until main test signals to start
                if let Err(e) = runner(platform).await {
                    log::error!("Error in managed run: {}", e);
                }
            }
        });

        // Wait a bit to allow the run function to start
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        Self {
            platform,
            calls,
            handle: Some(handle),
            notify,
        }
    }

    pub async fn stop_and_wait_task(&mut self, timeout_secs: u64) -> Result<()> {
        self.platform.get_stop().set();
        let handle = self.handle.take().unwrap(); // Fail if already taken
        tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), handle)
            .await
            .map_err(|e| {
                println!("Timeout waiting for run task to finish: {}", e);
                std::fmt::Error
            })?
            .map_err(|e| {
                println!("Error in run task: {}", e);
                std::fmt::Error
            })?;
        Ok(())
    }
}
