use anyhow::Result;
use std::{pin::Pin, sync::Arc};
use tokio::sync::Notify;

use shared::{log, service::AsyncService, config::{ActorType}};

mod platform;
mod managed;
mod unamanaged;

fn executor(stop: Arc<Notify>) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {
        let platform = platform::Platform::new(); // If no config, panic, we need config
        async_main(platform, stop).await.unwrap_or_else(|e| {
            log::error!("Error in async_main: {}", e);
        });
    })
}

fn main() {
    // Setup logging
    log::setup_logging("info", log::LogType::Service);

    // Install default ring provider for rustls
    if rustls::crypto::ring::default_provider()
        .install_default()
        .is_err()
    {
        log::error!("Failed to install default ring provider for rustls");
        return;
    }

    // Create the async launcher with our main async function
    let launcher = AsyncService::new(executor);

    // Run the service (on Windows) or directly (on other OS)
    if let Err(e) = launcher.run_service() {
        log::error!("Service failed to run: {}", e);
    }
}

// Real "main" async logic of the service
async fn async_main(platform: platform::Platform, stop: Arc<Notify>) -> Result<()> {
    log::info!("Service main async logic started");

    let cfg = platform.config().read().await.clone();
    if !cfg.is_valid() {
        log::error!("Invalid configuration, cannot start service");
        return Err(anyhow::anyhow!("Invalid configuration, cannot start service"));
    }

    if cfg.actor_type == ActorType::Unmanaged {
        log::info!("Starting in Unmanaged mode");
        unamanaged::run(platform.clone(), stop.clone()).await?;
    } else {
        log::info!("Starting in Managed mode");
        managed::run(platform.clone(), stop.clone()).await?;
    }

    let broker = platform.broker_api();
    log::debug!("Platform initialized with config: {:?}", platform.config());

    let interfaces = platform.operations().get_network_info()?;
    broker
        .write()
        .await
        .initialize(interfaces.as_slice())
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

#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod tests;