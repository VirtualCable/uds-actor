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
use std::io;
use std::mem;
use std::net::Ipv4Addr;
use std::os::raw::{c_char, c_int};
use std::ptr;

use libc::{self, sockaddr};

use anyhow::Result;

use crate::{log, operations::NetworkInterface};

/// Returns iterator (Vec) of InterfaceInfo for “valid” interfaces.
use std::ffi::CStr;
use std::net::Ipv4Addr;
use libc::{getifaddrs, freeifaddrs, ifaddrs, AF_INET, AF_LINK, sockaddr_in, sockaddr_dl};

#[derive(Debug)]
pub struct NetworkInterface {
    pub name: String,
    pub ip_addr: String,
    pub mac: String,
}

pub fn get_network_info() -> Result<Vec<NetworkInterface>> {
    let mut ifaces: *mut ifaddrs = std::ptr::null_mut();
    let mut result = Vec::new();

    unsafe {
        if getifaddrs(&mut ifaces) != 0 {
            return result;
        }

        let mut cur = ifaces;
        while !cur.is_null() {
            let ifa = &*cur;

            if !ifa.ifa_addr.is_null() {
                let name = CStr::from_ptr(ifa.ifa_name).to_string_lossy().into_owned();
                let family = (*ifa.ifa_addr).sa_family as i32;

                if family == AF_INET {
                    // Dirección IPv4
                    let sa = &*(ifa.ifa_addr as *const sockaddr_in);
                    let ip = Ipv4Addr::from(u32::from_be(sa.sin_addr.s_addr));
                    result.push(NetworkInterface {
                        name,
                        ip_addr: ip.to_string(),
                        mac: String::new(), // se rellena en AF_LINK
                    });
                } else if family == AF_LINK {
                    // Dirección MAC
                    let sdl = &*(ifa.ifa_addr as *const sockaddr_dl);
                    let mac_bytes = std::slice::from_raw_parts(
                        sdl.sdl_data.as_ptr().offset(sdl.sdl_nlen as isize) as *const u8,
                        sdl.sdl_alen as usize,
                    );
                    let mac = mac_bytes
                        .iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<_>>()
                        .join(":");

                    result.push(NetworkInterface {
                        name,
                        ip_addr: String::new(),
                        mac,
                    });
                }
            }

            cur = (*cur).ifa_next;
        }

        freeifaddrs(ifaces);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_interfaces() {
        log::setup_logging("debug", crate::log::LogType::Tests);
        let names = list_interfaces().unwrap();
        assert!(!names.is_empty());
        for name in &names {
            log::info!("Interface: {}", name);
        }
    }

    #[test]
    fn test_get_ipv4_addr() {
        log::setup_logging("debug", crate::log::LogType::Tests);
        let names = list_interfaces().unwrap();
        for name in &names {
            if let Some(ip) = get_ipv4_addr(name) {
                log::info!("Interface: {}, IPv4: {}", name, ip);
            }
        }
    }

    #[test]
    fn test_get_mac_addr() {
        log::setup_logging("debug", crate::log::LogType::Tests);
        let names = list_interfaces().unwrap();
        for name in &names {
            if let Some(mac) = get_mac_addr(name) {
                log::info!("Interface: {}, MAC: {}", name, mac);
            }
        }
    }

    #[test]
    fn test_get_network_info() {
        log::setup_logging("debug", crate::log::LogType::Tests);
        let infos = get_network_info();
        assert!(infos.is_ok());
        for info in &infos.unwrap() {
            log::info!(
                "Interface: {}, IP: {}, MAC: {}",
                info.name,
                info.ip_addr,
                info.mac
            );
        }
    }
}
