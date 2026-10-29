use super::*;
use serde_json::json;

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
async fn test_managed_stops_when_initialize_fails() -> Result<()> {
    let mut test_setup = TestSetup::new(run).await;
    test_setup.broker_api.write().await.init_error = Some("broker refused".into());
    // Signal the run function to start
    test_setup.notify.notify_one();

    test_setup.stop_and_wait_task(1).await?;

    log::info!("Calls: {:?}", test_setup.calls.dump());
    assert!(test_setup.calls.count_calls("broker_api::initialize") == 1);
    assert!(test_setup.calls.count_calls("broker_api::ready") == 0);
    Ok(())
}

#[tokio::test]
#[serial_test::serial(server)]
async fn test_managed_resumes_when_broker_refuses_own_token() -> Result<()> {
    let mut test_setup = TestSetup::new(run).await;
    {
        let config = test_setup.platform.config();
        let mut cfg = config.write().await;
        cfg.master_token = None;
        cfg.own_token = Some("stored_own_token".into());
        cfg.config.os = Some(shared::config::ActorOsConfiguration {
            action: shared::config::ActorOsAction::Rename,
            name: "stored_name".into(),
            custom: Some(json!({
                "username": "user",
                "password": "old",
                "new_password": "new",
            })),
        });
    }
    test_setup.broker_api.write().await.init_error = Some("Access denied".into());
    test_setup.notify.notify_one();

    test_setup.stop_and_wait_task(1).await?;

    log::info!("Calls: {:?}", test_setup.calls.dump());
    assert!(test_setup.calls.count_calls("broker_api::initialize") == 1);
    assert!(
        test_setup
            .calls
            .count_calls("broker_api::set_token(stored_own_token)")
            >= 1
    );
    assert!(test_setup.calls.count_calls("operations::rename_computer") == 0);
    test_setup
        .calls
        .assert_not_called("operations::change_user_password");
    assert!(test_setup.calls.count_calls("broker_api::ready") == 1);
    Ok(())
}

#[tokio::test]
#[serial_test::serial(server)]
async fn test_managed_stops_on_other_broker_errors_with_own_token() -> Result<()> {
    let mut test_setup = TestSetup::new(run).await;
    {
        let config = test_setup.platform.config();
        let mut cfg = config.write().await;
        cfg.master_token = None;
        cfg.own_token = Some("stored_own_token".into());
    }
    test_setup.broker_api.write().await.init_error = Some("Internal Server Error".into());
    test_setup.notify.notify_one();

    test_setup.stop_and_wait_task(1).await?;

    log::info!("Calls: {:?}", test_setup.calls.dump());
    assert!(test_setup.calls.count_calls("broker_api::initialize") == 1);
    assert!(test_setup.calls.count_calls("broker_api::ready") == 0);
    Ok(())
}

#[tokio::test]
#[serial_test::serial(server)]
async fn test_managed_stops_when_broker_refuses_master_token() -> Result<()> {
    let mut test_setup = TestSetup::new(run).await;
    {
        let config = test_setup.platform.config();
        let mut cfg = config.write().await;
        cfg.master_token = Some("master_token".into());
        cfg.own_token = Some("stale_own_token".into());
    }
    test_setup.broker_api.write().await.init_error = Some("Access denied".into());
    test_setup.notify.notify_one();

    test_setup.stop_and_wait_task(1).await?;

    log::info!("Calls: {:?}", test_setup.calls.dump());
    assert!(test_setup.calls.count_calls("broker_api::initialize") == 1);
    assert!(test_setup.calls.count_calls("broker_api::ready") == 0);
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
async fn test_managed_rename_with_new_password_changes_password() -> Result<()> {
    let mut test_setup = TestSetup::new(run).await;
    let computer_name = "new_actor_name".to_string();
    test_setup.broker_api.write().await.init_response =
        shared::broker::api::types::InitializationResponse {
            master_token: Some("mastertoken".into()),
            token: Some("owntoken".into()),
            unique_id: Some("uniqueid".into()),
            os: Some(shared::config::ActorOsConfiguration {
                action: shared::config::ActorOsAction::Rename,
                name: computer_name.clone(),
                custom: Some(json!({
                    "username": "demouser",
                    "password": "demouser",
                    "new_password": "NewP@ssw0rd!"
                })),
            }),
        };
    test_setup.notify.notify_one();
    test_setup.stop_and_wait_task(1).await?;

    log::info!("Calls: {:?}", test_setup.calls.dump());
    // change_user_password must run with the credentials the OS manager supplied
    assert!(
        test_setup
            .calls
            .count_calls("operations::change_user_password(demouser,demouser,NewP@ssw0rd!)")
            == 1,
        "change_user_password was not invoked with the OS manager payload"
    );
    assert!(test_setup.calls.count_calls("operations::rename_computer") == 1);
    assert!(test_setup.calls.count_calls("operations::reboot") == 1);
    Ok(())
}

#[tokio::test]
#[serial_test::serial(server)]
async fn test_managed_rename_without_new_password_skips_password_change() -> Result<()> {
    let mut test_setup = TestSetup::new(run).await;
    test_setup.broker_api.write().await.init_response =
        shared::broker::api::types::InitializationResponse {
            master_token: Some("mastertoken".into()),
            token: Some("owntoken".into()),
            unique_id: Some("uniqueid".into()),
            os: Some(shared::config::ActorOsConfiguration {
                action: shared::config::ActorOsAction::Rename,
                name: "new_actor_name".into(),
                custom: Some(json!({ "username": "demouser" })),
            }),
        };
    test_setup.notify.notify_one();
    test_setup.stop_and_wait_task(1).await?;

    log::info!("Calls: {:?}", test_setup.calls.dump());
    test_setup
        .calls
        .assert_not_called("operations::change_user_password");
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
    test_setup
        .calls
        .assert_not_called("operations::rename_computer");
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
    assert!(
        test_setup
            .calls
            .count_calls("operations::ensure_domain_membership")
            == 1
    );
    assert!(test_setup.calls.count_calls("operations::reboot") == 1); // Should reboot after join
    Ok(())
}

#[tokio::test]
#[serial_test::serial(server)]
async fn test_managed_join_domain_failure_reports_to_broker() -> Result<()> {
    use std::sync::atomic::Ordering;

    let mut test_setup = TestSetup::new(run).await;
    // Force the OS-level join to fail (mimics NetJoinDomain Win32 error 2).
    test_setup
        .operations
        .fail_join_domain
        .store(true, Ordering::SeqCst);
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
    // Note: a failed OS action should hit broker /log and abort before reboot
    // or ready(), otherwise it just restarts with an empty "Registros" panel.
    assert!(test_setup.calls.count_calls("operations::ensure_domain_membership") == 1);
    assert!(test_setup.calls.count_calls("broker_api::log") == 1);
    test_setup.calls.assert_not_called("operations::reboot");
    test_setup.calls.assert_not_called("broker_api::ready");
    Ok(())
}

#[tokio::test]
#[serial_test::serial(server)]
async fn test_managed_rename_failure_reports_to_broker() -> Result<()> {
    use std::sync::atomic::Ordering;

    let mut test_setup = TestSetup::new(run).await;
    test_setup
        .operations
        .fail_rename
        .store(true, Ordering::SeqCst);
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
    assert!(test_setup.calls.count_calls("operations::rename_computer") == 1);
    test_setup.calls.assert_not_called("operations::reboot");
    assert!(test_setup.calls.count_calls("broker_api::log") == 1);
    test_setup.calls.assert_not_called("broker_api::ready");
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
    test_setup
        .calls
        .assert_not_called("operations::rename_computer");
    // ensure_domain_membership IS called (delegates the "already joined?" check
    // to the platform). The mock returns Ok(false), so no reboot.
    assert!(
        test_setup
            .calls
            .count_calls("operations::ensure_domain_membership")
            == 1
    );
    test_setup.calls.assert_not_called("operations::reboot");
    Ok(())
}
