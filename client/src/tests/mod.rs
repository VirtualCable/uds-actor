use crate::testing::dummy::create_platform;

#[tokio::test]
async fn test_run_no_server() {
    // Execute run function. As long as there is no server running on localhost, it will fail to login (Before registering)
    let platform = crate::platform::Platform::new(); // Default platform, no fake api
    let session_manager = platform.session_manager();

    let res = tokio::time::timeout(std::time::Duration::from_secs(4), super::run(platform)).await;
    shared::log::info!("Run finished with result: {:?}", res);

    // Stop should not be set, as run should fail to login and stop the session
    assert!(session_manager.is_running().await);
}

#[tokio::test]
async fn test_run_and_stop() {
    shared::log::setup_logging("debug", shared::log::LogType::Tests);
    // Start a mock server to allow login
    let (platform, _calls) = create_platform(None, None, None, None).await;

    let session_manager = platform.session_manager();

    assert!(session_manager.is_running().await);

    // Run on a separate task to be able to stop it, but use a timeout to avoid hanging forever
    let run_handle = tokio::spawn(async move {
        let res =
            tokio::time::timeout(std::time::Duration::from_secs(8), super::run(platform)).await;
        shared::log::info!("Run finished with result: {:?}", res);
    });

    // Wait a bit to ensure run has started and logged in
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    assert!(session_manager.is_running().await);
    // Now stop the session
    session_manager.stop().await;
    shared::log::info!("Session stop requested");
    // Wait for run to finish
    let _ = run_handle.await;
    assert!(!session_manager.is_running().await);
}
