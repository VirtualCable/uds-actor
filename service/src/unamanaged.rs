use std::sync::Arc;
use tokio::sync::Notify;

use anyhow::Result;

use shared::{log, ws::server};

use crate::platform;

pub async fn run(platform: platform::Platform, stop: Arc<Notify>) -> Result<()> {
    let broker = platform.broker_api();
    log::debug!("Platform initialized with config: {:?}", platform.config());

    let known_interfaces = platform.operations().get_network_info()?;

    let cert_info = broker
        .write()
        .await
        .unmanaged_ready(known_interfaces.as_slice(), shared::consts::UDS_PORT)
        .await
        .map_err(|e| {
            log::error!("Failed to initialize with broker: {:?}", e);
            anyhow::anyhow!("Failed to initialize with broker: {:?}", e)
        })?;

    let _http_server = server::start_server(
        cert_info.certificate.into(),
        cert_info.key.into(),
        cert_info.password,
        stop.clone(),
        platform.broker_api().read().await.get_secret().unwrap().to_string(),
    );

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
