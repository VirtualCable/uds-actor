use std::sync::Arc;

use anyhow::Result;

use shared::{log, sync::OnceSignal};

use crate::platform;

async fn wait_for_no_installation(platform: &platform::Platform) -> Result<()> {
    loop {
        if !platform.operations().is_some_installation_in_progress()? {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
    Ok(())
}

pub async fn run(platform: platform::Platform, stop: Arc<OnceSignal>) -> Result<()> {
    log::info!("Unmanaged service starting");

    // First, wait until no installation is runnning and no stop is requested
    // This is needed because on managed actors, the runonce command can
    // be used to run installations on first boot, (i.e. sysprep on windows)
    tokio::select! {
        _ = wait_for_no_installation(&platform) => {},
        _ = stop.wait() => {
            log::info!("Stop received before installation is complete; exiting");
            return Ok(());
        }
    }

    let broker = platform.broker_api();
    log::debug!("Platform initialized with config: {:?}", platform.config());

    // force time sync on managed startup
    if let Err(e) = platform.operations().force_time_sync() {
        log::warn!("Failed to force time sync on startup: {}", e);
    }

    // Call initialize with broker if not already initialized.
    // We know that we are already initialized if we have an own_token
    if platform.config().read().await.own_token.is_some() {
        log::info!("Unmanaged actor already initialized, skipping initialization");
    } else {
        log::info!("Unmanaged actor not initialized, initializing with broker");
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
    }

    // Here, we have to ensure the requested state (on os data) is the actual state of the machine


    Ok(())
}
