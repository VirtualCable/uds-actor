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
use widestring::{U16CStr, U16CString, U16String};
use windows::{
    Win32::{
        Foundation::{CloseHandle, GetLastError, HANDLE, WIN32_ERROR},
        NetworkManagement::IpHelper::{
            GET_ADAPTERS_ADDRESSES_FLAGS, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
        },
        Networking::WinSock::AF_INET,
        Security::{
            AdjustTokenPrivileges, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED, SE_SHUTDOWN_NAME,
            TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
        },
        System::{
            Diagnostics::Debug::{FORMAT_MESSAGE_FROM_SYSTEM, FormatMessageW},
            Shutdown::{
                EWX_FORCEIFHUNG, EWX_LOGOFF, EWX_REBOOT, EXIT_WINDOWS_FLAGS, ExitWindowsEx,
                SHUTDOWN_REASON,
            },
            SystemInformation::{
                COMPUTER_NAME_FORMAT, ComputerNamePhysicalDnsHostname, GetComputerNameExW,
                GetTickCount, GetVersionExW, OSVERSIONINFOW, SetComputerNameExW,
            },
            Threading::{GetCurrentProcess, OpenProcessToken},
            WindowsProgramming::GetUserNameW,
        },
        UI::Input::KeyboardAndMouse::{GetLastInputInfo, LASTINPUTINFO},
    },
    core::{PCWSTR, PWSTR},
};

use crate::log;

unsafe fn utf16_ptr_to_string(ptr: *const u16) -> String {
    if ptr.is_null() {
        return "<unknown>".to_string();
    }
    // Reinterpret the pointer as a null-terminated UTF-16 string
    let u16cstr = unsafe { U16CStr::from_ptr_str(ptr) };
    u16cstr.to_string_lossy()
}

pub fn check_permissions() -> bool {
    // Use IsUserAnAdmin from shell32
    use windows::Win32::UI::Shell::IsUserAnAdmin;
    unsafe { IsUserAnAdmin().as_bool() }
}

pub fn get_error_message(result_code: WIN32_ERROR) -> String {
    let mut buf = [0u16; 512];
    unsafe {
        let len = FormatMessageW(
            FORMAT_MESSAGE_FROM_SYSTEM,
            None,
            result_code.0,
            0,
            PWSTR(buf.as_mut_ptr()),
            buf.len() as u32,
            None,
        );
        U16String::from_vec(buf[..len as usize].to_vec()).to_string_lossy()
    }
}

pub fn get_computer_name() -> String {
    let mut buf = [0u16; 512];
    let mut size = buf.len() as u32;
    unsafe {
        if GetComputerNameExW(
            COMPUTER_NAME_FORMAT(5),
            Some(PWSTR(buf.as_mut_ptr())),
            &mut size,
        )
        .is_ok()
        {
            utf16_ptr_to_string(buf.as_ptr())
        } else {
            String::new()
        }
    }
}

fn get_network_info() -> Vec<(String, String, String)> {
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
        return vec![];
    }

    let mut results = vec![];
    unsafe {
        while !adapters_ptr.is_null() {
            let adapter = &*adapters_ptr;

            let name = if !adapter.FriendlyName.is_null() {
                utf16_ptr_to_string(adapter.FriendlyName.0)
            } else {
                "<unknown>".to_string()
            };

            let mac = (0..adapter.PhysicalAddressLength)
                .map(|i| format!("{:02X}", adapter.PhysicalAddress[i as usize]))
                .collect::<Vec<_>>()
                .join(":");

            // Aquí podrías recorrer adapter.FirstUnicastAddress para obtener IPs

            results.push((name, mac, "<ip>".to_string())); // IP real omitida por simplicidad

            adapters_ptr = adapter.Next;
        }
    }

    results
}

pub fn get_domain_name() -> String {
    String::new()
}

pub fn get_windows_version() -> (u32, u32, u32, u32, String) {
    unsafe {
        let mut info = OSVERSIONINFOW {
            dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32,
            ..Default::default()
        };
        if GetVersionExW(&mut info).is_ok() {
            let sz_cstr = utf16_ptr_to_string(info.szCSDVersion.as_ptr());
            (
                info.dwMajorVersion,
                info.dwMinorVersion,
                info.dwBuildNumber,
                info.dwPlatformId,
                sz_cstr,
            )
        } else {
            (0, 0, 0, 0, String::new())
        }
    }
}

pub fn get_version() -> String {
    let (major, minor, build, _platform, csd) = get_windows_version();
    format!("Windows-{}.{} Build {} ({})", major, minor, build, csd)
}

pub fn reboot(flags: Option<EXIT_WINDOWS_FLAGS>) {
    // On tests, return early to not reboot the test machine :D
    log::debug!("Reboot called with flags: {:?}", flags);

    if cfg!(test) {
        return;
    }

    let flags = flags.unwrap_or(EWX_FORCEIFHUNG | EWX_REBOOT);
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
}

pub fn logoff() {
    log::debug!("Logoff called");
    // On tests, return early to not logoff the test machine :D
    if cfg!(test) {
        return;
    }
    unsafe {
        _ = ExitWindowsEx(EWX_LOGOFF, SHUTDOWN_REASON(0));
    }
}

pub fn rename_computer(new_name: &str) -> bool {
    // On tests, return early to not rename the test machine :D
    if cfg!(test) {
        return true;
    }
    let wname = U16CString::from_str(new_name).unwrap();
    unsafe {
        if SetComputerNameExW(ComputerNamePhysicalDnsHostname, PCWSTR(wname.as_ptr())).is_ok() {
            true
        } else {
            let error = get_error_message(GetLastError());
            log::error!("Failed to rename computer: {}", error);
            false
        }
    }
}

pub fn get_idle_duration() -> f64 {
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
            millis as f64 / 1000.0
        } else {
            0.0
        }
    }
}

pub fn get_current_user() -> String {
    let mut buf = [0u16; 256];
    let mut size = buf.len() as u32;
    unsafe {
        if GetUserNameW(Some(PWSTR(buf.as_mut_ptr())), &mut size).is_ok() {
            utf16_ptr_to_string(buf.as_ptr())
        } else {
            String::new()
        }
    }
}

pub fn get_session_type() -> String {
    let env_var = std::env::var("SESSIONNAME");
    if let Ok(session_name) = env_var {
        return session_name;
    }
    {
        log::warn!("SESSIONNAME environment variable is not set");
        "unknown".to_string()
    }
}
