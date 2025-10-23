use anyhow::Result;

use shared::{config::ActorOsAction, log, ws::server};

use crate::{common, platform, workers};

pub async fn run(platform: platform::Platform) -> Result<()> {
    log::info!("Managed service starting");

    // Ensure we have all requisites to start
    common::wait_for_readyness(&platform).await?;

    log::debug!("Platform initialized with config: {:?}", platform.config());

    // force time sync on managed startup
    if let Err(e) = platform.operations().force_time_sync() {
        log::warn!("Failed to force time sync on startup: {}", e);
    }

    // Call initialize with broker if not already initialized.
    if platform.config().read().await.already_initialized() {
        log::info!("Managed actor already initialized, skipping initialization");
    } else if let Err(e) = crate::common::initialize(&platform).await {
        log::error!("Failed to initialize managed actor with broker: {}", e);
        return Err(anyhow::anyhow!(
            "Failed to initialize managed actor with broker: {}",
            e
        ));
    }

    if crate::computer::process_command(&platform, crate::computer::CommandType::RunOnce).await {
        // If runonce was executed, exit
        log::info!("Exiting after runonce execution as requested");
        return Ok(());
    }

    if let Some(os_data) = platform.config().read().await.config.os.clone() {
        match os_data.action {
            ActorOsAction::None => {
                log::debug!("No OS action requested");
            }
            ActorOsAction::Rename => {
                log::info!("OS action requested: Rename to '{}'", os_data.name);
                if crate::computer::rename_computer(&platform, os_data.name.as_str()).await? {
                    // Reboot to apply changes
                    log::info!("Rebooting system to apply rename");
                    platform.operations().reboot(None)?;
                    return Ok(()); // We can exit here, system is rebooting
                }
                // Already has the correct name, skips reboot
            }
            ActorOsAction::JoinDomain => {
                log::info!(
                    "OS action requested: Join domain with name '{}'",
                    os_data.name
                );
                if crate::computer::join_domain(
                    &platform,
                    os_data.name.as_str(),
                    os_data.custom.clone(),
                )
                .await?
                {
                    // Reboot to apply changes
                    log::info!("Rebooting system to apply domain join");
                    platform.operations().reboot(None)?;
                    return Ok(()); // We can exit here, system is rebooting
                }
                // Already has the correct name and domain, skips reboot
            }
        }
    } else {
        log::debug!("No OS data action requested");
    }

    // Post-config command will run, but no reboot will be done after it
    crate::computer::process_command(&platform, crate::computer::CommandType::PostConfig).await;

    // Notify ready to broker, will return TLS certs
    // Note: The server is started after this, as we need the certs to start it
    // Is not expected to receive any calls before server is started (and will not)
    let broker = platform.broker_api();
    let ip = platform
        .operations()
        .get_network_info()?
        .first()
        .cloned()
        .map(|ni| ni.ip_addr)
        .unwrap_or_default();

    let cert_info = broker
        .write()
        .await
        .ready(ip.as_str(), shared::consts::UDS_PORT)
        .await
        .map_err(|e| {
            log::error!("Failed to initialize with broker: {:?}", e);
            anyhow::anyhow!("Failed to initialize with broker: {:?}", e)
        })?;

    // Spawn the webserver/websocket server
    // Initialize the Webserver/Websocket server (webserver for public part, websocket for local client comms)
    let (server_info, _server_task) = server::start_server(
        cert_info.clone(),
        platform.get_stop(),
        platform
            .broker_api()
            .read()
            .await
            .get_secret()
            .unwrap()
            .to_string(),
        None, // Default port
    )
    .await?;

    // create the ip watcher task
    // Will simply stop the service if ip changes
    // Allowing the system to restart it cleanly
    tokio::spawn({
        let platform = platform.clone();
        async move {
            if let Err(e) = common::interfaces_watch_task(&platform, None).await {
                log::error!("Error in interfaces watch task: {}", e);
            }
        }
    });

    // Create workers for requests, wsclient communication, etc.
    workers::create_workers(server_info.clone(), platform.clone()).await;

    // Simply wait here until stop is signaled
    platform.get_stop().wait().await;
    log::info!("Managed service stopping");
    Ok(())
}

#[cfg(test)]
mod tests;