use shared::sync::OnceSignal;

use crate::testing::mock::mock_platform;

#[tokio::test]
async fn test_run_no_server() {
    shared::log::setup_logging("debug", shared::log::LogType::Tests);
    shared::tls::init_tls(None);
    // Execute run function. As long as there is no server running on localhost, it will fail to login (Before registering)
    assert!(crate::platform::Platform::new(43910).await.is_err());
}

#[tokio::test]
async fn test_run_and_stop() {
    shared::log::setup_logging("debug", shared::log::LogType::Tests);
    // Start a mock server to allow login
    // Get real session manager for this test
    let stop = OnceSignal::new();
    let session_manager = crate::session::new_session_manager(stop.clone()).await;
    let (platform, _calls, _, _) =
        mock_platform(Some(session_manager), None, None, Some(stop.clone()), 43910).await;

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

#[tokio::test]
async fn test_run_and_stop_via_platform() {
    shared::log::setup_logging("debug", shared::log::LogType::Tests);
    // Start a mock server to allow login
    // Get real session manager for this test
    let stop = OnceSignal::new();
    let session_manager = crate::session::new_session_manager(stop.clone()).await;
    let (platform, _calls, _, _) =
        mock_platform(Some(session_manager), None, None, Some(stop.clone()), 43910).await;

    let session_manager = platform.session_manager();

    assert!(session_manager.is_running().await);

    // Run on a separate task to be able to stop it, but use a timeout to avoid hanging forever
    let run_handle = tokio::spawn(async move {
        // Wait a bit to allow session_manager to fully start
        let res =
            tokio::time::timeout(std::time::Duration::from_secs(8), super::run(platform)).await;
        shared::log::info!("Run finished with result: {:?}", res);
    });

    // Wait a bit to allow full initialization
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;

    stop.set();

    tokio::time::sleep(std::time::Duration::from_secs_f32(0.1)).await;

    // Just ensure session is stopped
    if !session_manager.is_running().await {
        session_manager.stop().await;
        shared::log::info!("Session should be stopped");
    }

    shared::log::info!("Session stop requested");
    stop.set();
    // Wait for run to finish
    let _ = run_handle.await;
    shared::log::info!("Run has finished");
}
