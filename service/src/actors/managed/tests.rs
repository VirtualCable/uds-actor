use super::*;

use crate::actors::testing::TestSetup;

#[tokio::test]
#[serial_test::serial(server)]
async fn test_managed_basic_and_stop() -> Result<()> {
    let mut test_setup = TestSetup::new(run).await;
    // Signal the run function to start
    test_setup.notify.notify_one();

    test_setup.stop_and_wait_task(1).await?;

    log::info!("Calls: {:?}", test_setup.calls.dump());
    assert!(test_setup.calls.count_calls("operations::force_time_sync") == 1);
    assert!(test_setup.calls.count_calls("broker_api::initialize") == 1);
    assert!(test_setup.calls.count_calls("broker_api::ready") == 1);
    Ok(())
}

#[tokio::test]
#[serial_test::serial(server)]
async fn test_managed_already_initialized() -> Result<()> {
    let mut test_setup = TestSetup::new(run).await;
    // Set already_initialized to true
    test_setup.platform.config().write().await.own_token = Some("mastertoken".into());
    // Signal the run function to start
    test_setup.notify.notify_one();
    test_setup.stop_and_wait_task(1).await?;

    log::info!("Calls: {:?}", test_setup.calls.dump());
    assert!(test_setup.calls.count_calls("operations::force_time_sync") == 1);
    assert!(test_setup.calls.count_calls("broker_api::initialize") == 0); // Should not call initialize
    assert!(test_setup.calls.count_calls("broker_api::ready") == 1);

    Ok(())
}
