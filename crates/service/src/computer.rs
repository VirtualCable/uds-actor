// Copyright (c) 2025 Virtual Cable S.L.U.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
//
//    * Redistributions of source code must retain the above copyright notice,
//      this list of conditions and the following disclaimer.
//    * Redistributions in binary form must reproduce the above copyright notice,
//      this list of conditions and the following disclaimer in the documentation
//      and/or other materials provided with the distribution.
//    * Neither the name of Virtual Cable S.L.U. nor the names of its contributors
//      may be used to endorse or promote products derived from this software
//      without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
/*!
Author: Adolfo Gómez, dkmaster at dkmon dot com
*/
use std::fmt::Display;

use anyhow::Result;
use shared::system;

use crate::common;
use crate::platform;

use crate::log;

/// Rename the computer to the specified name.
/// Accepts only hostname labels that are safe for line-oriented root-owned config
/// files (/etc/hostname, /etc/hosts, HOSTNAME=): single-line, bounded, and built
/// from Word characters only — ASCII alphanumerics, hyphen and underscore
/// (underscore is valid in Windows/NetBIOS names and used across this repo's own
/// fixtures). Control characters, whitespace and colons are rejected, which is the
/// injection-proof invariant; strict RFC-1123 DNS labels would also reject the
/// underscore, breaking legitimate Windows renames.
fn is_valid_hostname(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 253
        && name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                && !label.starts_with('-')
                && !label.ends_with('-')
        })
}

/// Returns Ok(true) if the name was changed and a reboot is required,
/// Ok(false) if the name was already the current name (no change),
pub async fn rename_computer(platform: &platform::Platform, name: &str) -> Result<bool> {
    if !is_valid_hostname(name) {
        anyhow::bail!(
            "invalid computer name '{name}': must contain only ASCII letters, digits, '-', '_' and '.' (no control characters or whitespace)"
        );
    }
    log::info!("Renaming system to '{}'", name);
    // If the name is already the current name, skip
    let op = platform.system();

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

    // Parse custom data, extract possible required fields
    let custom = custom.unwrap();
    let join_options = system::JoinDomainOptions {
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
            .filter(|s| !s.is_empty())
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

    // Rename the machine first (may return Ok(true) if a reboot will be needed)
    let renamed = rename_computer(platform, name).await?;

    // Delegate the "is the trust healthy?" decision to the platform, so each
    // OS can do the appropriate validation/repair (e.g. Windows can probe
    // and repair the secure channel without rebooting).
    //
    // ensure_domain_membership returns Ok(true) when changes were applied
    // that require a reboot. We must reboot in that case.
    let ops = platform.system();
    let opts = join_options.clone();
    let needs_reboot =
        tokio::task::spawn_blocking(move || ops.ensure_domain_membership(&opts)).await??;

    // If the machine was just renamed, a reboot is required anyway for the
    // rename to take effect (handled by the managed caller).
    Ok(needs_reboot || renamed)
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
pub async fn process_command(
    platform: &platform::Platform,
    command_type: CommandType,
    args: &[&str],
) -> bool {
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
            common::run_command(command_type.to_string().as_str(), run_cmd.as_str(), args).await
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

#[cfg(test)]
mod hostname_validation_tests {
    use super::is_valid_hostname;

    #[test]
    fn accepts_windows_compatible_names() {
        assert!(is_valid_hostname("new_actor_name"));
        assert!(is_valid_hostname("PC-01.lab.local"));
        assert!(is_valid_hostname("a"));
    }

    #[test]
    fn rejects_injection_shapes() {
        assert!(!is_valid_hostname(""));
        assert!(!is_valid_hostname("name\n10.0.0.1 evil"));
        assert!(!is_valid_hostname("name with spaces"));
        assert!(!is_valid_hostname("name:colon"));
        assert!(!is_valid_hostname("name\rtab"));
    }

    #[test]
    fn rejects_out_of_bounds_labels() {
        assert!(!is_valid_hostname(&"a".repeat(254)));
        assert!(!is_valid_hostname(&format!("{}.local", "a".repeat(64))));
        assert!(!is_valid_hostname("-leading-hyphen"));
        assert!(!is_valid_hostname("trailing-."));
    }
}
