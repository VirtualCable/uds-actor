use super::*;

use crate::log::{LogType, info, setup_logging};

#[test]
fn test_check_permissions() {
    setup_logging("debug", LogType::Tests);
    let ops = new_operations();
    let result = ops.check_permissions();
    assert!(result.is_ok());
    // We are not admin, should be false
    assert!(!result.unwrap());
}

#[test]
fn test_get_computer_name() {
    let env_name = std::env::var("COMPUTERNAME").unwrap();
    let ops = new_operations();
    let result = ops.get_computer_name();
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), env_name);
}

#[test]
fn test_get_domain_name() {
    setup_logging("debug", LogType::Tests);
    let ops = new_operations();
    let result = ops.get_domain_name();
    assert!(result.is_ok());
    // Domain name can be empty if not joined to a domain
    let _domain_name = result.unwrap();
}

// rename_computer is not tested to avoid renaming the test machine

// join_domain is not tested to avoid joining the test machine to a domain

// change_user_password is not tested to avoid changing any user password

#[test]
fn test_get_os_version() {
    setup_logging("debug", LogType::Tests);
    let ops = new_operations();
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
    let ops = new_operations();
    let result = ops.init_idle_timer();
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
    let ops = new_operations();
    let result = ops.get_current_user();
    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user, env_user);
    info!("Current user: {}", user);
}

#[test]
fn test_get_session_type() {
    setup_logging("debug", LogType::Tests);
    let ops = new_operations();
    let result = ops.get_session_type();
    assert!(result.is_ok());
    let session_type = result.unwrap();
    assert!(!session_type.is_empty());
    info!("Session type: {}", session_type);
}

#[test]
fn test_get_network_info() {
    setup_logging("debug", LogType::Tests);
    let ops = new_operations();
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
fn test_force_time_sync() {
    setup_logging("debug", LogType::Tests);
    // Check if we are admin
    let ops = new_operations();
    let perm = ops.check_permissions().unwrap_or(false);
    let result = ops.force_time_sync();
    info!("force_time_sync result: {}", result.is_ok());

    assert!(result.is_ok() == perm);
}

#[test]
fn test_protect_file_for_owner_only() {
    setup_logging("debug", LogType::Tests);
    let ops = new_operations();
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
fn test_is_some_installation_in_progress() {
    setup_logging("debug", LogType::Tests);
    let ops = new_operations();
    let result = ops.is_some_installation_in_progress();
    assert!(result.is_ok());
    let in_progress = result.unwrap();
    info!("Is some installation in progress: {}", in_progress);
    // We can't assert the value, just that it returned ok
}