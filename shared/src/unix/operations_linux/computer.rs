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
    ffi::CStr,
    io::{self, Write},
    process::{Command, Stdio},
};

use anyhow::Result;

use crate::log;

pub(super) fn get_computer_name() -> Result<String> {
    // Tipical maximum hostname length
    const HOST_NAME_MAX: usize = 255;
    let mut buf = [0u8; HOST_NAME_MAX];

    // libc::gethostname
    // Also available on /proc/sys/kernel/hostname but using libc is more direct
    let ret = unsafe { libc::gethostname(buf.as_mut_ptr() as *mut i8, buf.len()) };
    if ret != 0 {
        return Err(io::Error::last_os_error().into());
    }

    let cstr = unsafe { CStr::from_ptr(buf.as_ptr() as *const i8) };
    let hostname = cstr.to_string_lossy().into_owned();

    // Cortar por el primer '.'
    let short = hostname.split('.').next().unwrap_or(&hostname);
    Ok(short.to_string())
}

pub(super) fn join_domain(options: &crate::operations::JoinDomainOptions) -> Result<()> {
    log::debug!("Joining domain with options: {:?}", options);

    let domain = options.domain.clone();
    let ou = options.ou.clone();
    let account = options.account.clone();
    let password = options.password.clone();
    let client_software = options.client_software.as_deref().unwrap_or_default();
    let server_software = options.server_software.as_deref().unwrap_or_default();
    let membership_software = options.membership_software.as_deref().unwrap_or_default();
    let ssl = options.ssl.unwrap_or(false);
    let automatic_id_mapping = options.automatic_id_mapping.unwrap_or(false);

    // FreeIPA: adjust hostname
    if server_software == "ipa" {
        if let Ok(hostname) = get_computer_name() {
            let fqdn = format!("{}.{}", hostname.to_lowercase(), domain);
            log::debug!("Setting hostname for FreeIPA: {}", fqdn);
            if let Err(e) = Command::new("hostnamectl")
                .arg("set-hostname")
                .arg(&fqdn)
                .status()
            {
                log::error!("Error setting hostname for freeipa: {e}");
            }
        }
    }

    // Build realm join command
    let mut cmd = Command::new("realm");
    cmd.arg("join").arg(format!("--user={}", account));

    if !client_software.is_empty() && client_software != "automatically" {
        cmd.arg(format!("--client-software={}", client_software));
    }
    if !server_software.is_empty() {
        cmd.arg(format!("--server-software={}", server_software));
    }
    if !membership_software.is_empty() && membership_software != "automatically" {
        cmd.arg(format!("--membership-software={}", membership_software));
    }
    if let Some(ou) = ou.as_ref() {
        if !ou.is_empty() && server_software != "ipa" {
            cmd.arg(format!("--computer-ou={}", ou));
        }
    }
    if ssl {
        cmd.arg("--use-ldaps");
    }
    if automatic_id_mapping {
        cmd.arg("--automatic-id-mapping=no");
    }

    cmd.arg(&domain);

    log::debug!("Joining domain {} with command: {:?}", domain, cmd);

    // Pasar la contraseña por stdin
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(password.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    if output.status.success() {
        log::debug!("Joined domain {} successfully", domain);
    } else {
        log::error!(
            "Error joining domain {}: {}",
            domain,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}

fn is_timesyncd_active() -> Result<bool> {
    let status = Command::new("systemctl")
        .arg("is-active")
        .arg("systemd-timesyncd")
        .output()?;

    Ok(status.status.success() && String::from_utf8_lossy(&status.stdout).trim() == "active")
}

/// Ensures that the system time is updated by restarting systemd-timesyncd if it is active.
pub(super) fn refresh_system_time() -> Result<()> {
    if is_timesyncd_active()? {
        log::debug!("systemd-timesyncd is active, restarting to force time sync");
        let status = Command::new("systemctl")
            .arg("restart")
            .arg("systemd-timesyncd")
            .status()?;
        if status.success() {
            log::debug!("Local time updated via systemd-timesyncd");
            Ok(())
        } else {
            log::error!("Failed to restart systemd-timesyncd");
            Err(anyhow::anyhow!(
                "systemctl restart systemd-timesyncd failed"
            ))
        }
    } else {
        log::warn!("systemd-timesyncd is not active, cannot refresh time");
        Err(anyhow::anyhow!("systemd-timesyncd not active"))
    }
}
