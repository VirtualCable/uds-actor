use anyhow::Result;

use shared::{log, ws::server};

use crate::{common, platform, workers};

pub async fn run(platform: platform::Platform) -> Result<()> {
    log::info!("Unmanaged service starting");

    // Ensure we have all requisites to start
    common::wait_for_readyness(&platform).await?;

    log::debug!("Platform initialized with config: {:?}", platform.config());

    // force time sync on managed startup
    if let Err(e) = platform.operations().force_time_sync() {
        log::warn!("Failed to force time sync on startup: {}", e);
    }

    // Call initialize with broker if not already initialized.
    if platform.config().read().await.already_initialized() {
        log::info!("Unmanaged actor already initialized, skipping initialization");
    } else if let Err(e) = crate::common::initialize(&platform).await {
        log::error!("Failed to initialize unmanaged actor with broker: {}", e);
        return Err(anyhow::anyhow!(
            "Failed to initialize unmanaged actor with broker: {}",
            e
        ));
    }

    // Here, we have to ensure the requested state (on os data) is the actual state of the machine
    // 1.- If runonce is pending, run it, clear on config AND EXIT (the script must reboot by)
    // 2.- execute corresponding action for os data

    let cfg = platform.config().read().await.clone();
    if let Some(run_once) = cfg.runonce_command.clone() {
        log::info!("Runonce script pending, executing: {}", run_once);
        if let Err(e) = common::run_command("run_once", run_once.as_str(), &[]).await {
            log::error!("Failed to execute runonce script {}: {}", run_once, e);
        } else {
            log::info!("Runonce script {} executed successfully", run_once);
            // Clear run_once on config
            let cfg = platform.config(); // Avoid drop while writing
            let mut cfg = cfg.write().await;
            cfg.runonce_command = None;
            let mut saver = platform.config_storage();
            if let Err(e) = saver.save_config(&cfg) {
                log::error!("Failed to save config after clearing run_once: {}", e);
            }
        }
        log::info!("Exiting after runonce execution as requested");
        return Ok(());
    }

    // TODO: handle os data actions here

    // Notify ready to broker, will return TLS certs
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
    let server_info = server::start_server(
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
    log::info!("Unmanaged service stopping");
    Ok(())
}
