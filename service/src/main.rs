use std::sync::{Arc, OnceLock};

use shared::service::AsyncService;

use tokio::sync::Notify;

static STOP: OnceLock<Arc<Notify>> = OnceLock::new();

async fn async_main() {
    let stop = STOP.get().expect("STOP not initialized").clone();
    // Main async logic here
    shared::log::info!("Service main async logic started");
    let start = std::time::Instant::now();
    loop {
        tokio::select! {
            _ = stop.notified() => {
                shared::log::info!("Stop received in async_main; exiting");
                break;
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                shared::log::info!("Service is running... {}", start.elapsed().as_millis());
            }
            // ...tus I/O, servidores, etc., también deberían cooperar con stop
        }
    }
}

fn main() {
    // Setup logging
    shared::log::setup_logging("info", shared::log::LogType::Service);

    let stop = Arc::new(Notify::new());
    STOP.set(stop.clone()).unwrap();
    // Create the async launcher with our main async function
    let launcher = AsyncService::new(|| Box::pin(async_main()), stop.clone());

    // Run the service (on Windows) or directly (on other OS)
    if let Err(e) = launcher.run_service() {
        shared::log::error!("Service failed to run: {}", e);
    }
}
