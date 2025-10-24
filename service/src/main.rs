use anyhow::Result;
use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use shared::{
    config::ActorType,
    installer, log,
    service::{AsyncService, AsyncServiceTrait},
    sync::OnceSignal,
    tls,
};

mod actors;
mod common;
mod computer;
mod platform;

mod workers;

fn executor(
    stop: OnceSignal,
    restart_flag: Arc<AtomicBool>,
) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> {
    Box::pin(async move {
        let platform = platform::Platform::new(stop, restart_flag); // If no config, panic, we need config
        async_main(platform).await
    })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        println!("Service installer options detected: {}", args[1]);
        match args[1].as_str() {
            "--install" => {
                if let Err(e) = installer::register(
                    "UDSActorService",
                    "UDS Actor Service",
                    "UDS Actor Management Service",
                ) {
                    eprintln!("Failed to install service: {}", e);
                } else {
                    println!("Service installed successfully.");
                }
            }
            "--uninstall" => {
                if let Err(e) = installer::unregister("UDSActorService") {
                    eprintln!("Failed to uninstall service: {}", e);
                } else {
                    println!("Service uninstalled successfully.");
                }
            }
            _ => {
                eprintln!("Unknown option: {}", args[1]);
                eprintln!("Usage: {} [--install|--uninstall]", args[0]);
            }
        }
        std::process::exit(1);
    }

    // Setup logging
    log::setup_logging("info", log::LogType::Service);
    log::info!("***** Starting UDS Actor Service *****");

    tls::init_tls(None);

    // Create the async launcher with our main async function
    let launcher = AsyncService::new(executor);
    let restart_flag = launcher.get_restart_flag();

    // Run the service (on Windows) or directly (on other OS)
    // Note that run_service will block until service stops
    // On linux, it a systemd service
    // On macOS, it is a launchd service
    // On Windows, it is a Windows service
    if let Err(e) = launcher.run_service() {
        log::error!("Service failed to run: {}", e);
    }

    if restart_flag.load(Ordering::Relaxed) {
        log::info!("Service requested restart, exiting with specific code");
        std::process::exit(1); // Exit with code 1 to indicate restart
    } else {
        log::info!("Service exited normally");
    }
}

// Real "main" async logic of the service
async fn async_main(platform: platform::Platform) -> Result<()> {
    log::info!("Service main async logic started");
    // Setup logging level from config
    let log_level = platform.config().read().await.log_level();
    log::set_log_level(log_level.into());
    log::info!("Logging level set to: {:?}", log_level);

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
        actors::unmanaged::run(platform.clone()).await?;
    } else {
        log::info!("Starting in Managed mode");
        actors::managed::run(platform.clone()).await?;
    }
    log::info!("Service main async logic exiting");
    Ok(())
}

#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod tests;
