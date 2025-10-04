use std::{pin::Pin, sync::Arc};

use shared::{log, service::AsyncService};

use tokio::sync::Notify;

mod rest;

fn executor(stop: Arc<Notify>) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    Box::pin(async move {
        async_main(stop).await;
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

async fn async_main(stop: Arc<Notify>) {
    log::info!("Service main async logic started");
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
}
