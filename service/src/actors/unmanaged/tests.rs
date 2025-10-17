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
    assert!(test_setup.calls.count_calls("broker_api::unmanaged_ready") == 1);
    Ok(())
}
