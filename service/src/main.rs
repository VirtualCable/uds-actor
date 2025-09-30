use shared::launcher::AsyncLauncher;

async fn async_main() {
    // Main async logic here
    shared::log::info!("Service main async logic started");
    // For example, run some server or perform tasks
    // Here we just sleep for demonstration
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        shared::log::info!("Service is running...");
    }
}

fn main() {
    // Setup logging
    shared::log::setup_logging("info", shared::log::LogType::Service);

    // Create the async launcher with our main async function
    let launcher = AsyncLauncher::new(|| Box::pin(async_main()));

    // Run the service (on Windows) or directly (on other OS)
    if let Err(e) = launcher.run_service() {
        shared::log::error!("Service failed to run: {}", e);
    }
}
