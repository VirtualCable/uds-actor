// Copyright (c) 2025 Virtual Cable S.L.U.
// All rights reserved.
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
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
use std::{
    io::Write,
    process::{Command, Stdio},
};

use anyhow::Result;

use crate::log;

mod computer;
mod idle;
mod network;
mod renamer;
mod session;

pub fn new_operations() -> std::sync::Arc<dyn crate::operations::Operations + Send + Sync> {
    std::sync::Arc::new(LinuxOperations::new())
}

pub struct LinuxOperations;

impl LinuxOperations {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_linux_version(&self) -> Option<String> {
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("ID=") {
                    return Some(line[3..].trim_matches('"').to_string());
                }
            }
        }
        None
    }
}

// TODO: Implement remaining methods
impl crate::operations::Operations for LinuxOperations {
    fn check_permissions(&self) -> Result<()> {
        log::debug!("LinuxOperations::check_permissions called");
        if unsafe { libc::geteuid() != 0 } {
            Err(anyhow::anyhow!("Insufficient permissions"))
        } else {
            Ok(())
        }
    }

    fn get_computer_name(&self) -> Result<String> {
        log::debug!("LinuxOperations::get_computer_name called");
        computer::get_computer_name()
    }

    fn get_domain_name(&self) -> Result<Option<String>> {
        log::debug!("LinuxOperations::get_domain_name called");
        Ok(None)
    }

    fn rename_computer(&self, new_name: &str) -> Result<()> {
        log::debug!("LinuxOperations::rename_computer called: {}", new_name);
        renamer::renamer(
            new_name,
            self.get_linux_version().as_deref().unwrap_or("unknown"),
        )
    }

    fn join_domain(&self, options: &crate::operations::JoinDomainOptions) -> Result<()> {
        computer::join_domain(options)
    }

    fn change_user_password(
        &self,
        user: &str,
        _old_password: &str,
        new_password: &str,
    ) -> Result<()> {
        log::debug!("LinuxOperations::change_user_password called: user={}", user);

        // chpasswd expects "user:new_password" in stdin
        let input = format!("{}:{}\n", user, new_password);

        let mut child = Command::new("/usr/sbin/chpasswd")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        if let Some(stdin) = child.stdin.as_mut() {
            stdin.write_all(input.as_bytes())?;
        }

        let output = child.wait_with_output()?;
        if output.status.success() {
            log::debug!("Password for {} changed successfully", user);
            Ok(())
        } else {
            log::error!(
                "Error changing password for {}: {}",
                user,
                String::from_utf8_lossy(&output.stderr)
            );
            Err(anyhow::anyhow!("chpasswd failed"))
        }
    }

    fn get_os_version(&self) -> Result<String> {
        log::debug!("LinuxOperations::get_os_version called");
        Ok(self
            .get_linux_version()
            .unwrap_or("generic-linux".to_string()))
    }

    fn reboot(&self, flags: Option<u32>) -> Result<()> {
        log::debug!("LinuxOperations::reboot called: {:?}", flags);
        Command::new("systemctl").arg("reboot").status()?;
        Ok(())
    }

    fn logoff(&self) -> Result<()> {
        log::debug!("LinuxOperations::logoff called");
        session::logout()
    }

    fn get_network_info(&self) -> Result<Vec<crate::operations::NetworkInterface>> {
        log::debug!("LinuxOperations::get_network_info called");
        network::get_network_info()
    }

    fn init_idle_timer(&self) -> Result<()> {
        log::debug!("LinuxOperations::init_idle_timer called");
        idle::init_idle()
    }

    fn get_idle_duration(&self) -> Result<std::time::Duration> {
        log::debug!("LinuxOperations::get_idle_duration called");
        let idle = idle::get_idle();
        Ok(std::time::Duration::from_secs_f64(idle))
    }

    fn get_current_user(&self) -> Result<String> {
        log::debug!("LinuxOperations::get_current_user called");
        Ok(whoami::username())
    }

    fn get_session_type(&self) -> Result<String> {
        log::debug!("LinuxOperations::get_session_type called");
        Ok(std::env::var("XRDP_SESSION").unwrap_or_else(|_| {
            std::env::var("XDG_SESSION_TYPE").unwrap_or_else(|_| "unknown".to_string())
        }))
    }

    fn force_time_sync(&self) -> Result<()> {
        log::debug!("LinuxOperations::force_time_sync called");
        computer::refresh_system_time()
    }

    fn protect_file_for_owner_only(&self, path: &str) -> Result<()> {
        log::debug!(
            "LinuxOperations::protect_file_for_owner_only called: {}",
            path
        );
        Ok(())
    }

    fn ensure_user_can_rdp(&self, user: &str) -> Result<()> {
        log::debug!("LinuxOperations::ensure_user_can_rdp called: {}", user);
        Ok(())
    }

    fn is_some_installation_in_progress(&self) -> Result<bool> {
        log::debug!("LinuxOperations::is_some_installation_in_progress called");
        Ok(false)
    }
}
