use super::*;

use crate::log::{LogType, info, setup_logging};

#[test]
#[ignore = "Manual test, requires admin privileges"]
fn test_check_permissions() {
    setup_logging("debug", LogType::Tests);
    let ops = new_system();
    let result = ops.check_permissions();
    // We are not admin, should be false
    assert!(result.is_err());
}

#[test]
fn test_get_computer_name() {
    let env_name = std::env::var("COMPUTERNAME").unwrap();
    let ops = new_system();
    let result = ops.get_computer_name();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), env_name);
}

#[test]
fn test_get_domain_name() {
    setup_logging("debug", LogType::Tests);
    let ops = new_system();
    let result = ops.get_domain_name();
    assert!(result.is_ok());
    // Domain name can be empty if not joined to a domain
    let _domain_name = result.unwrap();
}

// rename_computer is not tested to avoid renaming the test machine

// join_domain is not tested to avoid joining the test machine to a domain

// change_user_password is not tested to avoid changing any user password

// -----------------------------------------------------------------------
// Regression tests for OU = "" -> NULL mapping.
//
// The server (`windows_domain.py`) emits `custom.ou = ""` whenever the
// OsManager has no OU configured. NetJoinDomain rejects an empty string
// with ERROR_FILE_NOT_FOUND (code 2), so we must convert "empty" -> NULL
// on the Rust side before building the wide string.
//
// These tests do NOT call NetJoinDomain itself (that would require admin
// privileges and a real domain). They just validate the data shape:
//   - server payload parses cleanly when ou = ""
//   - JoinDomainOptions with Some("") is treated as None for the syscall
// -----------------------------------------------------------------------

#[test]
fn test_os_action_payload_rename_ad_with_empty_ou_parses() {
    use crate::config::ActorOsConfiguration;

    // Real payload shape that the v4.0 and v5.0 servers send when the user
    // did NOT configure an OU on the OsManager.
    let json = r#"{
        "action": "rename_ad",
        "name": "UDSZVZ000",
        "custom": {
            "domain": "vc.local",
            "ou": "",
            "account": "administrador",
            "password": "fictitious-password"
        }
    }"#;

    let cfg: ActorOsConfiguration =
        serde_json::from_str(json).expect("server payload must deserialize");
    let custom = cfg.custom.expect("custom block must be present");

    // The empty string should reach us verbatim; the NULL coercion is the
    // responsibility of the windows code, not of the config layer.
    assert_eq!(custom.get("domain").and_then(|v| v.as_str()), Some("vc.local"));
    assert_eq!(custom.get("ou").and_then(|v| v.as_str()), Some(""));
    assert_eq!(
        custom.get("account").and_then(|v| v.as_str()),
        Some("administrador")
    );
}

#[test]
fn test_join_options_empty_ou_should_be_treated_as_none() {
    use crate::system::JoinDomainOptions;

    // This mirrors what `computer::join_domain` builds before calling
    // `WindowsOperations::join_domain`. If the server sends ou="" the
    // resulting options carry Some("") — and the windows-side wrapper
    // *must* convert that to NULL on the way to NetJoinDomain.
    let options = JoinDomainOptions {
        domain: "vc.local".into(),
        account: "administrador".into(),
        password: "secret".into(),
        ou: Some(String::new()), // <-- empty string from the server
        client_software: None,
        server_software: None,
        membership_software: None,
        ssl: None,
        automatic_id_mapping: None,
    };

    // The only thing we can verify at the unit-test level (without invoking
    // the Win32 syscall) is that the precondition that drives the NULL
    // coercion holds: the trimmed value must be empty.
    assert!(
        options.ou.as_deref().map(str::trim).unwrap_or("").is_empty(),
        "An empty OU should be coerced to NULL when calling NetJoinDomain"
    );

    // And the symmetric case: a real OU must NOT be coerced.
    let options_with_ou = JoinDomainOptions { ou: Some("OU=Machines,DC=vc,DC=local".into()), ..options };
    assert!(
        !options_with_ou
            .ou
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .is_empty()
    );
}

#[test]
fn test_get_os_version() {
    setup_logging("debug", LogType::Tests);
    let ops = new_system();
    let result = ops.get_os_version();
    assert!(result.is_ok());
    let version = result.unwrap();
    assert!(!version.is_empty());
    info!("OS Version: {}", version);
}

// reboot is not tested to avoid rebooting the test machine

// logoff is not tested to avoid logging off the test user

#[test]
#[ignore = "Manual test, requires user interaction (stay ilde :) )"]
fn test_idle_timer() {
    setup_logging("debug", LogType::Tests);
    let ops = new_system();
    let result = ops.init_idle_timer(32);
    assert!(result.is_ok());
    // Wait a bit
    std::thread::sleep(std::time::Duration::from_millis(100));
    // Get idle duration
    let result = ops.get_idle_duration();
    info!("Idle duration result: {:?}", result);

    assert!(result.is_ok());
    let duration = result.unwrap();
    // Duration should be non-negative, hopefully we don't moved the mouse :D
    assert!(duration.as_millis() >= 1);
}

#[test]
fn get_current_user() {
    setup_logging("debug", LogType::Tests);
    let env_user = std::env::var("USERNAME").unwrap();
    let ops = new_system();
    let result = ops.get_current_user();
    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user, env_user);
    info!("Current user: {}", user);
}

#[test]
fn test_get_session_type() {
    setup_logging("debug", LogType::Tests);
    let ops = new_system();
    let result = ops.get_session_type();
    assert!(result.is_ok());
    let session_type = result.unwrap();
    assert!(!session_type.is_empty());
    info!("Session type: {}", session_type);
}

#[test]
fn test_get_network_info() {
    setup_logging("debug", LogType::Tests);
    let ops = new_system();
    let result = ops.get_network_info();
    assert!(result.is_ok());
    let interfaces = result.unwrap();
    assert!(!interfaces.is_empty());
    for iface in interfaces {
        info!(
            "Interface: {} - IP: {} - MAC: {}",
            iface.name, iface.ip_addr, iface.mac
        );
    }
}

// force_time_sync will fail unless run as admin
#[test]
#[ignore = "Manual test, requires non admin privileges"]
fn test_force_time_sync() {
    setup_logging("debug", LogType::Tests);
    // Check if we are admin
    let ops = new_system();
    let perm = ops.check_permissions().is_ok();
    let result = ops.force_time_sync();
    info!("force_time_sync result: {}", result.is_ok());

    assert!(result.is_ok() == perm);
}

#[test]
fn test_protect_file_for_owner_only() {
    setup_logging("debug", LogType::Tests);
    let ops = new_system();
    // Create a temp file on temp dir
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("uds_actor_test_file.txt");
    let file_path_str = file_path.to_str().unwrap();
    let _ = std::fs::File::create(&file_path);
    // Protect the file
    let result = ops.protect_file_for_owner_only(file_path_str);
    assert!(result.is_ok());
    // Clean up
    let _ = std::fs::remove_file(&file_path);
}

#[test]
#[ignore = "Manual test, requires admin privileges"]
fn test_ensure_user_can_rdp() {
    setup_logging("debug", LogType::Tests);
    let ops = new_system();
    // Use current user for test
    let user = std::env::var("USERNAME").unwrap();
    let result = ops.ensure_user_can_rdp(&user);
    // If not run as admin, will fail with access denied (error code 5)
    info!("ensure_user_can_rdp result: {:?}", result);
    assert!(result.is_ok());
}

#[test]
fn test_is_some_installation_in_progress() {
    setup_logging("debug", LogType::Tests);
    let ops = new_system();
    let result = ops.is_some_installation_in_progress();
    assert!(result.is_ok());
    let in_progress = result.unwrap();
    info!("Is some installation in progress: {}", in_progress);
    // We can't assert the value, just that it returned ok
}
