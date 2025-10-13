use anyhow::Result;
use std::{pin::Pin, sync::Arc};

use shared::{config::ActorType, log, service::AsyncService, tls, sync::OnceSignal};

mod platform;

mod managed;
mod unamanaged;

mod workers;

fn executor(stop: Arc<OnceSignal>) -> Pin<Box<dyn Future<Output = ()> + Send>> {
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

    tls::init_tls(None); // TODO: allow config of cert path

    // Create the async launcher with our main async function
    let launcher = AsyncService::new(executor);

    // Run the service (on Windows) or directly (on other OS)
    // Note that run_service will block until service stops
    // On linux, it a systemd service
    // On macOS, it is a launchd service
    // On Windows, it is a Windows service
    if let Err(e) = launcher.run_service() {
        log::error!("Service failed to run: {}", e);
    }
}

// Real "main" async logic of the service
async fn async_main(platform: platform::Platform, stop: Arc<OnceSignal>) -> Result<()> {
    log::info!("Service main async logic started");

    // Validate config. If no config, this will error out
    let cfg = platform.config().read().await.clone();
    if !cfg.is_valid() {
        log::error!("Invalid configuration, cannot start service");
        return Err(anyhow::anyhow!(
            "Invalid configuration, cannot start service"
        ));
    }

    if cfg.actor_type == ActorType::Unmanaged {
        log::info!("Starting in Unmanaged mode");
        unamanaged::run(platform.clone(), stop.clone()).await?;
    } else {
        log::info!("Starting in Managed mode");
        managed::run(platform.clone(), stop.clone()).await?;
    }

    log::info!("Service main async logic exiting");
    Ok(())
}

#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod tests;
