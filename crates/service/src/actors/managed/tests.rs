use super::*;
use serde_json::json;

use crate::{actors::testing::TestSetup};

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

#[tokio::test]
#[serial_test::serial(server)]
async fn test_managed_rename_should_rename() -> Result<()> {
    let mut test_setup = TestSetup::new(run).await;
    // Setup the retun value for initialize
    test_setup.broker_api.write().await.init_response =
        shared::broker::api::types::InitializationResponse {
            master_token: Some("mastertoken".into()),
            token: Some("owntoken".into()),
            unique_id: Some("uniqueid".into()),
            os: Some(shared::config::ActorOsConfiguration {
                action: shared::config::ActorOsAction::Rename,
                name: "new_actor_name".into(),
                custom: None,
            }),
        };
    // Signal the run function to start
    test_setup.notify.notify_one();
    test_setup.stop_and_wait_task(1).await?;

    log::info!("Calls: {:?}", test_setup.calls.dump());
    assert!(test_setup.calls.count_calls("operations::force_time_sync") == 1);
    assert!(test_setup.calls.count_calls("broker_api::initialize") == 1);
    assert!(test_setup.calls.count_calls("operations::rename_computer") == 1);
    assert!(test_setup.calls.count_calls("operations::reboot") == 1); // Should reboot after rename
    Ok(())
}

#[tokio::test]
#[serial_test::serial(server)]
async fn test_managed_rename_should_not_rename() -> Result<()> {
    let mut test_setup = TestSetup::new(run).await;
    let computer_name = test_setup.platform.system().get_computer_name()?;
    // Setup the retun value for initialize
    test_setup.broker_api.write().await.init_response =
        shared::broker::api::types::InitializationResponse {
            master_token: Some("mastertoken".into()),
            token: Some("owntoken".into()),
            unique_id: Some("uniqueid".into()),
            os: Some(shared::config::ActorOsConfiguration {
                action: shared::config::ActorOsAction::Rename,
                name: computer_name.clone(),
                custom: None,
            }),
        };
    // Signal the run function to start
    test_setup.notify.notify_one();
    test_setup.stop_and_wait_task(1).await?;

    log::info!("Calls: {:?}", test_setup.calls.dump());
    assert!(test_setup.calls.count_calls("operations::force_time_sync") == 1);
    assert!(test_setup.calls.count_calls("broker_api::initialize") == 1);
    test_setup.calls.assert_not_called("operations::rename_computer");
    test_setup.calls.assert_not_called("operations::reboot"); // Should  not reboot after rename
    Ok(())
}

#[tokio::test]
#[serial_test::serial(server)]
async fn test_managed_join_domain_should_join() -> Result<()> {
    let mut test_setup = TestSetup::new(run).await;
    // Setup the retun value for initialize
    test_setup.broker_api.write().await.init_response =
        shared::broker::api::types::InitializationResponse {
            master_token: Some("mastertoken".into()),
            token: Some("owntoken".into()),
            unique_id: Some("uniqueid".into()),
            os: Some(shared::config::ActorOsConfiguration {
                action: shared::config::ActorOsAction::JoinDomain,
                name: "new_actor_name".into(),
                custom: Some(json!({
                    "domain": "domain.local",
                    "ou": "OU=Computers,DC=domain,DC=local",
                    "account": "admin",
                    "password": "password"
                })),
            }),
        };
    // Signal the run function to start
    test_setup.notify.notify_one();
    test_setup.stop_and_wait_task(1).await?;

    log::info!("Calls: {:?}", test_setup.calls.dump());
    assert!(test_setup.calls.count_calls("operations::force_time_sync") == 1);
    assert!(test_setup.calls.count_calls("broker_api::initialize") == 1);
    assert!(test_setup.calls.count_calls("operations::rename_computer") == 1);
    assert!(test_setup.calls.count_calls("operations::get_domain_name") == 1);
    assert!(test_setup.calls.count_calls("operations::join_domain") == 1);
    assert!(test_setup.calls.count_calls("operations::reboot") == 1); // Should reboot after join
    Ok(())
}

#[tokio::test]
#[serial_test::serial(server)]
async fn test_managed_join_domain_should_not_join() -> Result<()> {
    let mut test_setup = TestSetup::new(run).await;
    // Note that 
    let domain_name = test_setup.platform.system().get_domain_name()?;
    let computer_name = test_setup.platform.system().get_computer_name()?;
    // Setup the retun value for initialize
    test_setup.broker_api.write().await.init_response =
        shared::broker::api::types::InitializationResponse {
            master_token: Some("mastertoken".into()),
            token: Some("owntoken".into()),
            unique_id: Some("uniqueid".into()),
            os: Some(shared::config::ActorOsConfiguration {
                action: shared::config::ActorOsAction::JoinDomain,
                name: computer_name.clone(),
                custom: Some(json!({
                    "domain": domain_name.clone(),
                    "ou": "OU=Computers,DC=domain,DC=local",
                    "account": "admin",
                    "password": "password"
                })),
            }),
        };
    // Signal the run function to start
    test_setup.notify.notify_one();
    test_setup.stop_and_wait_task(1).await?;

    log::info!("Calls: {:?}", test_setup.calls.dump());
    assert!(test_setup.calls.count_calls("operations::force_time_sync") == 1);
    assert!(test_setup.calls.count_calls("broker_api::initialize") == 1);
    test_setup.calls.assert_not_called("operations::rename_computer");
    test_setup.calls.assert_not_called("operations::join_domain");
    Ok(())
}
