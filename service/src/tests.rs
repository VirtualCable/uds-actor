#[cfg(test)]
use super::*;

use shared::log;

use crate::testing::mock::mock_platform;

#[tokio::test]
async fn test_async_main_stops_managed() {
    log::setup_logging("debug", log::LogType::Tests);

    let (platform, _calls) = mock_platform().await;
    platform.config().write().await.own_token = Some("dummy_token".to_string());
    platform.config().write().await.actor_type = ActorType::Managed;

    let stop = platform.get_stop();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        stop.set();
    });
    let result = async_main(platform).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_async_main_stops_unmanaged() {
    log::setup_logging("debug", log::LogType::Tests);

    let (platform, _calls) = mock_platform().await;
    platform.config().write().await.master_token = Some("dummy_token".to_string());
    platform.config().write().await.actor_type = ActorType::Unmanaged;

    let stop = platform.get_stop();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        stop.set();
    });
    let result = async_main(platform).await;
    assert!(result.is_ok());
}