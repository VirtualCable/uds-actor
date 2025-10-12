#[cfg(test)]
use super::*;

use crate::testing::dummy::create_dummy_platform;

#[tokio::test]
async fn test_async_main() {
    let stop = Arc::new(Notify::new());
    let stop_clone = stop.clone();
    tokio::spawn(async move {
         tokio::time::sleep(std::time::Duration::from_secs(1)).await;
         stop_clone.notify_one();
    });
    let (platform, _calls) = create_dummy_platform().await;
    let result = async_main(platform, stop).await;
    assert!(result.is_ok());
}
