use std::{sync::Arc};
use tokio::sync::Notify;

use anyhow::Result;

use shared::log;

use crate::platform;

pub async fn run(platform: platform::Platform, stop: Arc<Notify>) -> Result<()> {
    let broker = platform.broker_api();
    log::debug!("Platform initialized with config: {:?}", platform.config());

    // force time sync on managed startup
    if let Err(e) = platform.operations().force_time_sync() {
        log::warn!("Failed to force time sync on startup: {}", e);
    }

    let known_interfaces = platform.operations().get_network_info()?;
    broker
        .write()
        .await
        .initialize(known_interfaces.as_slice())
        .await
        .map_err(|e| {
            log::error!("Failed to initialize with broker: {:?}", e);
            anyhow::anyhow!("Failed to initialize with broker: {:?}", e)
        })?;

    let start = std::time::Instant::now();
    loop {
        tokio::select! {
            _ = stop.notified() => {
                log::info!("Stop received in async_main; exiting");
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                log::info!("Service is running... {}", start.elapsed().as_millis());
            }

        }
    }
    log::info!("Service main async logic exiting");
    Ok(())

   
}