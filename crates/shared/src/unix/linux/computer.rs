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

    // Cut by the first '.'
    let short = hostname.split('.').next().unwrap_or(&hostname);
    Ok(short.to_string())
}

/// Returns the realm this host is currently joined to, or `None` if it is not
/// joined to any.
///
/// Uses `realm list --name-only`, which prints the names of configured
/// (joined) realms, one per line. We return the first one.
///
/// `realm` is **mandatory**: the actor joins domains exclusively through
/// `realm join`, so if a host is in a domain `realm` was installed at join
/// time and must still be present. If the `realm` binary is missing we
/// propagate the error instead of returning `None` — returning `None` would
/// be a lie (it would make `ensure_domain_membership` re-join on every boot).
pub(super) fn get_domain_name() -> Result<Option<String>> {
    let output = match Command::new("realm")
        .arg("list")
        .arg("--name-only")
        .output()
    {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // `realm` missing on a host that should be in a domain is an
            // abnormal state (we installed it at join time). Log loudly and
            // propagate, so the caller can report it instead of silently
            // re-joining every boot.
            log::error!(
                "'realm' binary not found. This host claims to be managed by \
                 the actor but 'realmd' is not installed; cannot determine \
                 current domain. (Install 'realmd' / 'realm' to fix.)"
            );
            return Err(anyhow::anyhow!(
                "'realm' binary not found; realmd is required: {}",
                e
            ));
        }
        Err(e) => {
            log::error!("Failed to spawn 'realm list --name-only': {}", e);
            return Err(e.into());
        }
    };
    if !output.status.success() {
        log::error!(
            "'realm list --name-only' exited with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return Err(anyhow::anyhow!(
            "realm list --name-only failed with status {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let name = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|s| s.trim())
        .find(|s| !s.is_empty())
        .map(|s| s.to_string());
    Ok(name)
}

pub(super) fn join_domain(options: &crate::system::JoinDomainOptions) -> Result<()> {
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
    if server_software == "ipa"
        && let Ok(hostname) = get_computer_name()
    {
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
    if let Some(ou) = ou.as_ref()
        && !ou.is_empty()
        && server_software != "ipa"
    {
        cmd.arg(format!("--computer-ou={}", ou));
    }

    if ssl {
        cmd.arg("--use-ldaps");
    }
    if automatic_id_mapping {
        cmd.arg("--automatic-id-mapping=no");
    }

    cmd.arg(&domain);

    log::debug!("Joining domain {} with command: {:?}", domain, cmd);

    // use a child process to run the command, and pass the password via stdin
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
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        log::error!(
            "Error joining domain {} (exit {:?}): {}",
            domain,
            output.status.code(),
            stderr.trim()
        );
        Err(anyhow::anyhow!(
            "realm join {} failed (exit {:?}): {}",
            domain,
            output.status.code(),
            stderr.trim()
        ))
    }
}

pub(super) fn ensure_domain_membership(options: &crate::system::JoinDomainOptions) -> Result<bool> {
    // 1. Are we joined to the requested domain already?
    let current_domain = get_domain_name().map_err(|e| {
        log::error!(
            "ensure_domain_membership: cannot determine current realm for '{}': {}",
            options.domain,
            e
        );
        e
    })?;

    match current_domain {
        None => {
            log::info!(
                "Not joined to any realm, performing full join to '{}'",
                options.domain
            );
            join_domain(options)
                .map_err(|e| {
                    log::error!(
                        "Full realm join to '{}' failed: {}",
                        options.domain,
                        e
                    );
                    e
                })?;
            return Ok(true);
        }
        Some(current) if !current.eq_ignore_ascii_case(&options.domain) => {
            log::warn!(
                "Joined to '{}', but requested '{}'; performing full join \
                 (machine was enrolled to a different realm)",
                current,
                options.domain
            );
            join_domain(options)
                .map_err(|e| {
                    log::error!(
                        "Full realm join to '{}' (from '{}') failed: {}",
                        options.domain,
                        current,
                        e
                    );
                    e
                })?;
            return Ok(true);
        }
        Some(current) => {
            log::debug!(
                "Already joined to '{}', probing machine-account trust",
                current
            );
        }
    }

    // 2. We are in the right realm. Try to verify the trust cheaply.
    //
    //    `net ads testjoin` authenticates with the local machine secret; it is
    //    the only reliable "is the machine account still valid?" probe we can
    //    call without extra packages. It is only meaningful for AD memberships
    //    created via samba (`--membership-software=samba`), but if `net` is
    //    present we use it; a false negative (testjoin fails even though the
    //    real trust is fine) just costs an unnecessary re-join, which is
    //    acceptable for this rare best-effort path.
    //
    //    If `net` is not installed (e.g. IPA-only host), we cannot verify the
    //    trust cheaply and fall back to always re-joining. See
    //    notes/domain-linux.md for the full rationale.
    //
    //    There is no credential-less repair for a stale secret on Linux
    //    (changetrustpw authenticates with the stale secret and so fails too),
    //    so a failed testjoin always falls back to `realm join`.
    match Command::new("net").arg("ads").arg("testjoin").output() {
        Ok(out) if out.status.success() => {
            log::info!(
                "net ads testjoin OK, machine-account trust to '{}' is healthy",
                options.domain
            );
            return Ok(false);
        }
        Ok(out) => {
            // `net ads testjoin` writes most of its diagnostics to STDOUT, not
            // stderr. Capture both plus the exit code so the operator can see
            // why the trust probe failed (stale secret, no DC reachable, DNS,
            // clock skew, ...).
            log::warn!(
                "net ads testjoin FAILED (exit code {:?}); machine-account trust \
                 to '{}' is broken, performing full re-join.\n\
                 --- stdout ---\n{}\n\
                 --- stderr ---\n{}",
                out.status.code(),
                options.domain,
                String::from_utf8_lossy(&out.stdout).trim(),
                String::from_utf8_lossy(&out.stderr).trim()
            );
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // `net` missing. For a host enrolled via samba this is anomalous
            // (samba-common-tools should be installed); for an IPA-only host
            // it is expected. Either way we cannot probe and fall back to
            // re-join, but the severity differs so the operator can spot a
            // misconfigured AD image.
            if options
                .server_software
                .as_deref()
                .is_none_or(|s| s.eq_ignore_ascii_case("active-directory"))
            {
                log::warn!(
                    "'net' (samba-common-tools) not available on this AD-enrolled \
                     host; cannot probe machine-account trust cheaply, falling \
                     back to full realm join on every service start. \
                     Install 'samba-common-tools' to enable the cheap probe."
                );
            } else {
                log::info!(
                    "'net' not available (likely an IPA-only host); cannot probe \
                     machine-account trust, falling back to full realm join"
                );
            }
        }
        Err(e) => {
            log::warn!(
                "Failed to spawn 'net ads testjoin' ({}); cannot probe trust, \
                 falling back to full realm join",
                e
            );
        }
    }

    log::debug!(
        "ensure_domain_membership: performing full realm join to '{}' to \
         ensure trust",
        options.domain
    );
    join_domain(options).map_err(|e| {
        log::error!(
            "Full realm join to '{}' during ensure_domain_membership failed: {}",
            options.domain,
            e
        );
        e
    })?;
    Ok(true)
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