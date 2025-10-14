use std::sync::Arc;

use anyhow::Result;

use shared::{log, sync::OnceSignal, ws::server};

use crate::{platform, workers, common};

pub async fn run(platform: platform::Platform, stop: Arc<OnceSignal>) -> Result<()> {
    log::info!("Unmanaged service starting");
    // Unmanaged actor does not need to wait for installations to complete
    // as it should not be doing installations at all

    // Ensure we have all requisites to start
    common::wait_for_readyness(&platform, stop.clone()).await?;

    let broker = platform.broker_api();
    log::debug!("Platform initialized with config: {:?}", platform.config());

    // On unmanaged, we get all network interfaces
    let known_interfaces = platform.operations().get_network_info()?;

    // Notify the broker that we are ready and get the TLS certs
    let cert_info = broker
        .write()
        .await
        .unmanaged_ready(known_interfaces.as_slice(), shared::consts::UDS_PORT)
        .await
        .map_err(|e| {
            log::error!("Failed to initialize with broker: {:?}", e);
            anyhow::anyhow!("Failed to initialize with broker: {:?}", e)
        })?;

    // Initialize the Webserver/Websocket server (webserver for public part, websocket for local client comms)
    let server_info = server::start_server(
        cert_info.clone(),
        stop.clone(),
        platform
            .broker_api()
            .read()
            .await
            .get_secret()
            .unwrap()
            .to_string(),
        None,  // Default port
    ).await?;

    log::info!("Http server started");

    // create the ip watcher task
    // Will simply stop the service if ip changes
    // Allowing the system to restart it cleanly
    tokio::spawn({
        let platform = platform.clone();
        let stop = stop.clone();
        async move {
            if let Err(e) = common::interfaces_watch_task(&platform, stop, None).await {
                log::error!("Error in interfaces watch task: {}", e);
            }
        }
    });

    // Create workers for requests, wsclient communication, etc.
    workers::create_workers(server_info.clone(), platform.clone()).await;

    // Simply wait here until stop is signaled
    stop.wait().await;
    log::info!("Unmanaged service stopping");
    Ok(())
}
