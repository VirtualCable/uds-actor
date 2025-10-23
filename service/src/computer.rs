use std::fmt::Display;

use anyhow::Result;
use shared::operations;

use crate::common;
use crate::platform;

use crate::log;

/// Rename the computer to the specified name.
/// Returns Ok(true) if the name was changed and a reboot is required,
/// Ok(false) if the name was already the current name (no change),
pub async fn rename_computer(platform: &platform::Platform, name: &str) -> Result<bool> {
    log::info!("Renaming system to '{}'", name);
    // If the name is already the current name, skip
    let op = platform.operations();

    let current_name = op.get_computer_name()?;
    if current_name.eq_ignore_ascii_case(name) {
        log::info!("System name is already '{}', skipping rename", name);
        return Ok(false);
    }
    // Rename the computer on a blocking task to avoid blocking the async runtime
    let name_clone = name.to_string();
    tokio::task::spawn_blocking(move || op.rename_computer(name_clone.as_str())).await??;

    log::info!("System renamed successfully to '{}'", name);
    // A reboot is usually required for the change to take effect
    // Take care of it outside this function
    Ok(true)
}

pub async fn join_domain(
    platform: &platform::Platform,
    name: &str,
    custom: Option<serde_json::Value>,
) -> Result<bool> {
    if custom.is_none() {
        return Err(anyhow::anyhow!(
            "No custom data provided for join domain action"
        ));
    }
    let operations = platform.operations();

    // Parse custom data, extract possible required fields
    let custom = custom.unwrap();
    let join_options = operations::JoinDomainOptions {
        domain: custom
            .get("domain")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        account: custom
            .get("account")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        password: custom
            .get("password")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        ou: custom
            .get("ou")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        client_software: custom
            .get("client_software")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        server_software: custom
            .get("server_software")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        membership_software: custom
            .get("membership_software")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        ssl: custom.get("ssl").and_then(|v| v.as_bool()),
        automatic_id_mapping: custom.get("automatic_id_mapping").and_then(|v| v.as_bool()),
    };

    // Rename the machine first
    // Execute on a blocking task to avoid blocking the async runtime
    let renamed = rename_computer(platform, name).await?;

    // If already joined to the requested domain, and name not changed, skip
    if let Ok(Some(current_domain)) = operations.get_domain_name()
        && current_domain.eq_ignore_ascii_case(&join_options.domain)
        && !renamed
    {
        log::info!(
            "System is already joined to domain '{}', skipping join",
            current_domain
        );
        return Ok(false);
    }
    log::info!("Joining system to domain '{}'", join_options.domain);

    // Join the domain on a blocking task to avoid blocking the async runtime
    tokio::task::spawn_blocking(move || operations.join_domain(&join_options)).await??;

    // Again, a reboot is usually required for the change to take effect
    // Take care of it outside this function
    Ok(true)
}

// Process a command (pre_command, runonce_command, post_command)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandType {
    PreConnect,
    RunOnce,
    PostConfig,
}

impl Display for CommandType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandType::PreConnect => write!(f, "Pre-connect"),
            CommandType::RunOnce => write!(f, "Run-once"),
            CommandType::PostConfig => write!(f, "Post-connect"),
        }
    }
}

// Returns true if a command was executed, Ok(false) if no command was pending
pub async fn process_command(platform: &platform::Platform, command_type: CommandType) -> bool {
    // Note that if already initialized, runonce has already been executed and cleared
    let cfg = platform.config(); // Avoid drop while writing
    let mut cfg_guard = cfg.write().await;
    let cmd = match command_type {
        CommandType::PreConnect => &mut cfg_guard.pre_command,
        CommandType::RunOnce => &mut cfg_guard.runonce_command,
        CommandType::PostConfig => &mut cfg_guard.post_command,
    };
    if let Some(run_cmd) = cmd {
        log::info!("{} script pending, executing: {}", command_type, run_cmd);
        let mut success = false;
        if let Err(e) =
            common::run_command(command_type.to_string().as_str(), run_cmd.as_str(), &[]).await
        {
            log::error!(
                "Failed to execute {} script {}: {}",
                command_type,
                run_cmd,
                e
            );
        } else {
            log::info!("{} script {} executed successfully", command_type, run_cmd);
            success = true;
        }
        // Tried to execute, clear it, will not be executed again
        if command_type == CommandType::RunOnce {
            // Clear run_once on config
            cfg_guard.runonce_command = None;
            let mut saver = platform.config_storage();
            if let Err(e) = saver.save_config(&cfg_guard) {
                log::error!("Failed to save config after clearing run_once: {}", e);
            }
        }
        return success;
    }
    false
}
