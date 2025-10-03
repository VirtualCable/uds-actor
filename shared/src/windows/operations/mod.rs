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
//
// Author: Adolfo Gómez, dkmaster at dkmon dot com

#![allow(dead_code)]
use std::process::Command;

use anyhow::Context;
use widestring::{U16CStr, U16CString};
use windows::{
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        NetworkManagement::{
            IpHelper::{
                GET_ADAPTERS_ADDRESSES_FLAGS, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
            },
            NetManagement::{
                NETSETUP_ACCT_CREATE, NETSETUP_DOMAIN_JOIN_IF_JOINED, NETSETUP_JOIN_DOMAIN,
                NETSETUP_JOIN_WITH_NEW_NAME, NetApiBufferFree, NetGetJoinInformation,
                NetJoinDomain, NetSetupDomainName, NetSetupUnknownStatus, NetUserChangePassword,
            },
        },
        Networking::WinSock::AF_INET,
        Security::{
            ACL, ACL_REVISION, AddAccessAllowedAce, AdjustTokenPrivileges,
            Authorization::{SE_FILE_OBJECT, SetNamedSecurityInfoW},
            DACL_SECURITY_INFORMATION, InitializeAcl, LookupAccountNameW, LookupPrivilegeValueW,
            PSID, SE_PRIVILEGE_ENABLED, SE_SHUTDOWN_NAME, SidTypeUnknown, TOKEN_ADJUST_PRIVILEGES,
            TOKEN_PRIVILEGES, TOKEN_QUERY,
        },
        Storage::FileSystem::FILE_ALL_ACCESS,
        System::{
            Shutdown::{EWX_FORCEIFHUNG, EWX_LOGOFF, EWX_REBOOT, ExitWindowsEx, SHUTDOWN_REASON},
            SystemInformation::{
                ComputerNamePhysicalDnsHostname, GetComputerNameExW, GetTickCount, GetVersionExW,
                OSVERSIONINFOW, SetComputerNameExW,
            },
            Threading::{GetCurrentProcess, OpenProcessToken},
            WindowsProgramming::GetUserNameW,
        },
        UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
    },
    core::{PCWSTR, PWSTR},
};

use crate::{
    log,
    operations::{NetworkInterfaceInfo, Operations},
};

unsafe fn utf16_ptr_to_string(ptr: *const u16) -> anyhow::Result<String> {
    if ptr.is_null() {
        return Ok("<unknown>".to_string());
    }
    // Reinterpret the pointer as a null-terminated UTF-16 string
    let u16cstr = unsafe { U16CStr::from_ptr_str(ptr) };
    Ok(u16cstr.to_string_lossy())
}

pub fn new_operations() -> std::sync::Arc<dyn crate::operations::Operations + Send + Sync> {
    std::sync::Arc::new(WindowsOperations::new())
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WindowsOperations;

impl WindowsOperations {
    pub fn new() -> Self {
        Self {}
    }

    fn get_windows_version(&self) -> anyhow::Result<(u32, u32, u32, u32, String)> {
        unsafe {
            let mut info = OSVERSIONINFOW {
                dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
                ..Default::default()
            };
            if GetVersionExW(&mut info).is_ok() {
                let sz_cstr = utf16_ptr_to_string(info.szCSDVersion.as_ptr())?;
                Ok((
                    info.dwMajorVersion,
                    info.dwMinorVersion,
                    info.dwBuildNumber,
                    info.dwPlatformId,
                    sz_cstr,
                ))
            } else {
                Err(anyhow::anyhow!("GetVersionExW failed"))
            }
        }
    }
}

impl Operations for WindowsOperations {
    fn check_permissions(&self) -> anyhow::Result<bool> {
        // Use IsUserAnAdmin from shell32
        use windows::Win32::UI::Shell::IsUserAnAdmin;
        Ok(unsafe { IsUserAnAdmin().as_bool() })
    }

    fn get_computer_name(&self) -> anyhow::Result<String> {
        let mut buf = [0u16; 512];
        let mut size = buf.len() as u32;
        unsafe {
            if GetComputerNameExW(
                ComputerNamePhysicalDnsHostname,
                Some(PWSTR(buf.as_mut_ptr())),
                &mut size,
            )
            .is_ok()
            {
                // SAFETY: buf is populated by the Win32 call and null-terminated
                let s = utf16_ptr_to_string(buf.as_ptr())?;
                Ok(s)
            } else {
                Ok(String::new())
            }
        }
    }

    fn get_domain_name(&self) -> anyhow::Result<Option<String>> {
        unsafe {
            let mut buffer = PWSTR::null();
            let mut status = NetSetupUnknownStatus;

            // lpServer = None = local machine
            let ret = NetGetJoinInformation(None, &mut buffer, &mut status);

            if ret != 0 {
                return Err(anyhow::anyhow!("NetGetJoinInformation failed: {}", ret));
            }

            // Convert the returned PWSTR to String
            let domain = if !buffer.is_null() {
                let s = utf16_ptr_to_string(buffer.0 as *const u16)?;
                // Free memory allocated by NetGetJoinInformation
                NetApiBufferFree(Some(buffer.0 as _));
                s
            } else {
                String::new()
            };

            // Only return if joined to a domain
            if status == NetSetupDomainName {
                Ok(Some(domain))
            } else {
                Ok(None)
            }
        }
    }

    fn rename_computer(&self, new_name: &str) -> anyhow::Result<()> {
        let wname = U16CString::from_str(new_name)
            .context("failed to convert new computer name to UTF-16")?;
        unsafe {
            if let Err(e) =
                SetComputerNameExW(ComputerNamePhysicalDnsHostname, PCWSTR(wname.as_ptr()))
            {
                return Err(anyhow::anyhow!("Failed to rename computer: {}", e));
            }
        }
        Ok(())
    }

    fn join_domain(
        &self,
        options: &crate::operations::JoinDomainOptions,
    ) -> anyhow::Result<()> {
        log::debug!("WindowsOperations::join_domain called: options={:?}", options);
        unsafe {
            // Build user@domain style credentials if needed
            let mut account_str = options.account.to_string();
            if !account.contains('@') && !account.contains('\\') {
                if options.domain.contains('.') {
                    account_str = format!("{}@{}", options.account, options.domain);
                } else {
                    account_str = format!("{}\\{}", options.domain, options.account);
                }
            }

            // Flags
            let mut flags =
                NETSETUP_ACCT_CREATE | NETSETUP_DOMAIN_JOIN_IF_JOINED | NETSETUP_JOIN_DOMAIN;
            if options.execute_in_one_step.unwrap_or(true) {
                flags |= NETSETUP_JOIN_WITH_NEW_NAME;
            }

            // Convert to utf16
            let lp_domain =
                U16CString::from_str(options.domain).context("failed to convert domain to UTF-16")?;
            let lp_ou = match ou {
                Some(s) => Some(U16CString::from_str(s).context("failed to convert OU to UTF-16")?),
                None => None,
            };
            let lp_account = U16CString::from_str(&account_str)
                .context("failed to convert account to UTF-16")?;
            let lp_password =
                U16CString::from_str(options.password).context("failed to convert password to UTF-16")?;

            // Call
            let mut res = NetJoinDomain(
                PCWSTR::null(),
                PCWSTR(lp_domain.as_ptr()),
                lp_ou
                    .as_ref()
                    .map_or(PCWSTR::null(), |s| PCWSTR(s.as_ptr())),
                PCWSTR(lp_account.as_ptr()),
                PCWSTR(lp_password.as_ptr()),
                flags,
            );

            // If the error is "already joined", try again with less flags (no create account, use existing)
            // This may happen if the account already exists on another ou
            if res == 2224 {
                let flags = NETSETUP_DOMAIN_JOIN_IF_JOINED | NETSETUP_JOIN_DOMAIN;
                res = NetJoinDomain(
                    PCWSTR::null(),
                    PCWSTR(lp_domain.as_ptr()),
                    lp_ou
                        .as_ref()
                        .map_or(PCWSTR::null(), |s| PCWSTR(s.as_ptr())),
                    PCWSTR(lp_account.as_ptr()),
                    PCWSTR(lp_password.as_ptr()),
                    flags,
                );
            }

            if res == 0 {
                Ok(())
            } else {
                Err(anyhow::anyhow!("NetJoinDomain failed: {}", res))
            }
        }
    }

    fn change_user_password(
        &self,
        user: &str,
        old_password: &str,
        new_password: &str,
    ) -> anyhow::Result<()> {
        unsafe {
            let user_w = U16CString::from_str(user).context("invalid user UTF-16")?;
            let old_w =
                U16CString::from_str(old_password).context("invalid old password UTF-16")?;
            let new_w =
                U16CString::from_str(new_password).context("invalid new password UTF-16")?;

            let res = NetUserChangePassword(
                PCWSTR::null(), // NULL for local machine
                PCWSTR(user_w.as_ptr()),
                PCWSTR(old_w.as_ptr()),
                PCWSTR(new_w.as_ptr()),
            );

            if res == 0 {
                Ok(())
            } else {
                Err(anyhow::anyhow!("NetUserChangePassword failed: {}", res))
            }
        }
    }

    fn get_os_version(&self) -> anyhow::Result<String> {
        let (major, minor, build, _platform, csd) = self.get_windows_version()?;
        Ok(format!(
            "Windows-{}.{} Build {} ({})",
            major, minor, build, csd
        ))
    }

    fn reboot(&self, flags: Option<u32>) -> anyhow::Result<()> {
        log::debug!("Reboot called with flags: {:?}", flags);
        use windows::Win32::System::Shutdown::EXIT_WINDOWS_FLAGS;
        let wflags = flags.map(EXIT_WINDOWS_FLAGS);
        let flags = wflags.unwrap_or(EWX_FORCEIFHUNG | EWX_REBOOT);
        unsafe {
            let hproc = GetCurrentProcess();
            let mut htok = HANDLE::default();
            if OpenProcessToken(hproc, TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut htok).is_ok() {
                let mut tp = TOKEN_PRIVILEGES::default();
                let mut luid = Default::default();
                if LookupPrivilegeValueW(None, SE_SHUTDOWN_NAME, &mut luid).is_ok() {
                    tp.PrivilegeCount = 1;
                    tp.Privileges[0].Luid = luid;
                    tp.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
                    if let Err(e) = AdjustTokenPrivileges(htok, false, Some(&tp), 0, None, None) {
                        log::error!("Failed to adjust token privileges: {}", e.message());
                    }
                }
                _ = CloseHandle(htok);
            }
            _ = ExitWindowsEx(flags, SHUTDOWN_REASON(0));
        }
        Ok(())
    }

    fn logoff(&self) -> anyhow::Result<()> {
        log::debug!("Logoff called");
        unsafe {
            _ = ExitWindowsEx(EWX_LOGOFF, SHUTDOWN_REASON(0));
        }
        Ok(())
    }

    fn init_idle_timer(&self) -> anyhow::Result<()> {
        // Just a stub for compatibility with other OSes
        // On Windows, we don't need to initialize anything
        Ok(())
    }

    fn get_idle_duration(&self) -> anyhow::Result<std::time::Duration> {
        unsafe {
            let mut lii = LASTINPUTINFO {
                cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32,
                dwTime: 0,
            };
            if GetLastInputInfo(&mut lii as *mut _).as_bool() {
                let mut current: u64 = GetTickCount() as u64;
                let dwtime = lii.dwTime as u64;
                if current < dwtime {
                    current += 0x1_0000_0000; // Handle overflow of GetTickCount
                }
                let millis = current - dwtime;
                Ok(std::time::Duration::from_millis(millis))
            } else {
                Ok(std::time::Duration::from_secs(0))
            }
        }
    }

    fn get_current_user(&self) -> anyhow::Result<String> {
        log::debug!("Get current user called");
        let mut buf = [0u16; 256];
        let mut size = buf.len() as u32;
        unsafe {
            if GetUserNameW(Some(PWSTR(buf.as_mut_ptr())), &mut size).is_ok() {
                let s = utf16_ptr_to_string(buf.as_ptr())?;
                Ok(s)
            } else {
                Ok(String::new())
            }
        }
    }

    fn get_session_type(&self) -> anyhow::Result<String> {
        log::debug!("Get session type called");
        let env_var = std::env::var("SESSIONNAME");
        if let Ok(session_name) = env_var {
            return Ok(session_name);
        }
        log::warn!("SESSIONNAME environment variable is not set");
        Ok("unknown".to_string())
    }

    fn get_network_info(&self) -> anyhow::Result<Vec<NetworkInterfaceInfo>> {
        let mut buf_len: u32 = 32_768;
        let mut buf = vec![0u8; buf_len as usize];
        let mut adapters_ptr = buf.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH;

        let ret = unsafe {
            GetAdaptersAddresses(
                AF_INET.0 as _,
                GET_ADAPTERS_ADDRESSES_FLAGS(0),
                None,
                Some(adapters_ptr),
                &mut buf_len,
            )
        };

        if ret != 0 {
            return Err(anyhow::anyhow!("GetAdaptersAddresses failed: {}", ret));
        }

        let mut results = vec![];
        unsafe {
            while !adapters_ptr.is_null() {
                let adapter = &*adapters_ptr;

                let name = if !adapter.FriendlyName.is_null() {
                    match utf16_ptr_to_string(adapter.FriendlyName.0 as *const u16) {
                        Ok(s) => s,
                        Err(_) => "<unknown>".to_string(),
                    }
                } else {
                    "<unknown>".to_string()
                };

                let mac = (0..adapter.PhysicalAddressLength)
                    .map(|i| format!("{:02X}", adapter.PhysicalAddress[i as usize]))
                    .collect::<Vec<_>>()
                    .join(":");

                // Iterate FirstUnicastAddress to get IPs
                let mut addr = adapter.FirstUnicastAddress;
                loop {
                    if addr.is_null() {
                        break;
                    }
                    let sockaddr = (*addr).Address.lpSockaddr;
                    if (*sockaddr).sa_family == AF_INET {
                        // IPv4
                        let data = (*sockaddr).sa_data;
                        let ip = std::net::Ipv4Addr::new(
                            data[2] as u8,
                            data[3] as u8,
                            data[4] as u8,
                            data[5] as u8,
                        );
                        results.push(NetworkInterfaceInfo {
                            name: name.clone(),
                            ip_address: ip.to_string(),
                            mac: mac.clone(),
                        });
                    }
                    // Move to next unicast address
                    addr = (*addr).Next;
                }
                adapters_ptr = adapter.Next;
            }
        }

        Ok(results)
    }

    fn force_time_sync(&self) -> anyhow::Result<()> {
        log::debug!("Force time sync called");
        let status = Command::new(r"C:\Windows\System32\w32tm.exe")
            .arg("/resync")
            .status()?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("w32tm /resync failed with {:?}", status);
        }
    }

    fn protect_file_for_owner_only(&self, path: &str) -> anyhow::Result<()> {
        unsafe {
            // Convert path to UTF-16
            let path_w = U16CString::from_str(path)
                .context("failed to convert path to UTF-16 for SetNamedSecurityInfoW")?;

            // 1. Resolve the current user SID
            let mut sid: [u8; 256] = [0; 256];
            let mut sid_size = sid.len() as u32;
            let mut domain: [u16; 256] = [0; 256];
            let mut domain_size = domain.len() as u32;
            let mut sid_name_use = SidTypeUnknown;

            let user = self.get_current_user()?;
            let user_w =
                U16CString::from_str(&user).context("failed to convert username to UTF-16")?;

            LookupAccountNameW(
                None,
                PCWSTR(user_w.as_ptr()),
                Some(PSID(sid.as_mut_ptr() as _)),
                &mut sid_size,
                Some(PWSTR(domain.as_mut_ptr())),
                &mut domain_size,
                &mut sid_name_use,
            )
            .map_err(|e| anyhow::anyhow!("LookupAccountNameW failed: {}", e))?;

            // 2. Create ACL with an ACE that grants full access to the SID
            let mut acl_buf = vec![0u8; 1024];
            let acl = acl_buf.as_mut_ptr() as *mut ACL;
            InitializeAcl(acl, acl_buf.len() as u32, ACL_REVISION)
                .map_err(|e| anyhow::anyhow!("InitializeAcl failed: {}", e))?;

            AddAccessAllowedAce(
                acl,
                ACL_REVISION,
                FILE_ALL_ACCESS.0,
                PSID(sid.as_mut_ptr() as _),
            )
            .map_err(|e| anyhow::anyhow!("AddAccessAllowedAce failed: {}", e))?;

            // 3. Apply the new DACL to the file
            let err = SetNamedSecurityInfoW(
                PCWSTR(path_w.as_ptr()),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                None,
                None,
                Some(acl),
                None,
            );

            if err.0 != 0 {
                return Err(anyhow::anyhow!("SetNamedSecurityInfoW failed: {}", err.0));
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests;
