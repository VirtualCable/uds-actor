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
    consts, installer, log,
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
        let failed = match args[1].as_str() {
            "--install" => {
                remove_legacy_services();
                match installer::register(
                    consts::SERVICE_NAME,
                    consts::SERVICE_DISPLAY_NAME,
                    consts::SERVICE_DESCRIPTION,
                ) {
                    Ok(()) => {
                        println!("Service installed successfully.");
                        false
                    }
                    Err(e) => {
                        eprintln!("Failed to install service: {}", e);
                        true
                    }
                }
            }
            "--uninstall" => {
                remove_legacy_services();
                match installer::unregister(consts::SERVICE_NAME) {
                    Ok(()) => {
                        println!("Service uninstalled successfully.");
                        false
                    }
                    Err(e) => {
                        eprintln!("Failed to uninstall service: {}", e);
                        true
                    }
                }
            }
            _ => {
                eprintln!("Unknown option: {}", args[1]);
                eprintln!("Usage: {} [--install|--uninstall]", args[0]);
                true
            }
        };
        std::process::exit(if failed { 1 } else { 0 });
    }

    // Setup logging
    log::setup_logging("info", log::LogType::Service);
    log::info!("***** Starting UDS Actor Service *****");

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

fn remove_legacy_services() {
    for name in consts::LEGACY_SERVICE_NAMES {
        if let Err(e) = installer::unregister(name) {
            eprintln!("Could not remove legacy service {}: {}", name, e);
        }
    }
}

// Real "main" async logic of the service
async fn async_main(platform: platform::Platform) -> Result<()> {
    log::info!("Service main async logic started");

    let cfg = platform.config().read().await.clone();

    // Setup logging level from config
    let log_level = cfg.log_level();
    log::set_log_level(log_level.into());
    log::info!("Logging level set to: {:?}", log_level);

    // Initialize TLS with configured ciphers
    tls::init_tls(cfg.ssl_ciphers());

    // Validate config. If no config, this will error out
    if !cfg.is_valid() {
        log::error!("Invalid configuration, cannot start service");
        return Err(anyhow::anyhow!(
            "Invalid configuration, cannot start service"
        ));
    }

    let err = if cfg.actor_type == ActorType::Unmanaged {
        log::info!("Starting in Unmanaged mode");
        actors::unmanaged::run(platform.clone()).await
    } else {
        log::info!("Starting in Managed mode");
        actors::managed::run(platform.clone()).await
    };

    if let Err(e) = err {
        log::error!("Service main async logic exited with error: {}", e);
    } else {
        log::info!("Service main async logic exited");
    }

    // Release the Windows EventLog handle, if any. No-op on non-Windows
    // and on services where the layer was disabled. The OS reclaims the
    // handle on process exit anyway, but being explicit avoids leaking
    // a stale handle in long-lived services that are restart-managed.
    #[cfg(target_os = "windows")]
    shared::windows::eventlog::shutdown();

    Ok(())
}

#[cfg(test)]
pub mod testing;

#[cfg(test)]
mod tests;
