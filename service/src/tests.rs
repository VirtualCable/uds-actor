#[cfg(test)]
use super::*;

use shared::log;

use crate::testing::dummy::create_dummy_platform;

#[tokio::test]
async fn test_async_main() {
    log::setup_logging("debug", log::LogType::Tests);

    let (platform, _calls) = create_dummy_platform().await;
    platform.config().write().await.own_token = Some("dummy_token".to_string());

    let stop = platform.get_stop();
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        stop.set();
    });
    let result = async_main(platform).await;
    assert!(result.is_ok());
}
