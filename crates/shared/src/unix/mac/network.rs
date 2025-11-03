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
use std::net::Ipv4Addr;

use libc::{self};

use anyhow::Result;

use crate::system::NetworkInterface;

use libc::{
    AF_INET, AF_LINK, IFF_LOOPBACK, IFF_RUNNING, IFF_UP, freeifaddrs, getifaddrs, ifaddrs,
    sockaddr_dl, sockaddr_in,
};
/// Returns iterator (Vec) of InterfaceInfo for “valid” interfaces.
use std::ffi::CStr;

pub fn get_network_info() -> Result<Vec<NetworkInterface>> {
    let mut ifaces: *mut ifaddrs = std::ptr::null_mut();
    let mut result = Vec::new();

    unsafe {
        if getifaddrs(&mut ifaces) != 0 {
            return Ok(result);
        }

        let mut cur = ifaces;
        while !cur.is_null() {
            let ifa = &*cur;

            if !ifa.ifa_addr.is_null()
                && (ifa.ifa_flags & IFF_UP as u32) != 0
                && (ifa.ifa_flags & IFF_RUNNING as u32) != 0
                && (ifa.ifa_flags & IFF_LOOPBACK as u32) == 0
            {
                let name = CStr::from_ptr(ifa.ifa_name).to_string_lossy().into_owned();
                let family = (*ifa.ifa_addr).sa_family as i32;

                if family == AF_INET {
                    // IPv4 address
                    let sa = &*(ifa.ifa_addr as *const sockaddr_in);
                    let ip = Ipv4Addr::from(u32::from_be(sa.sin_addr.s_addr));
                    result.push(NetworkInterface {
                        name,
                        ip_addr: ip.to_string(),
                        mac: String::new(), // se rellena en AF_LINK
                    });
                } else if family == AF_LINK {
                    // MAC address
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

    // Now, we need to merge IP and MAC info for the same interface
    let mut merged_result: Vec<NetworkInterface> = Vec::new();
    for iface in result {
        if let Some(existing) = merged_result
            .iter_mut()
            .find(|i| i.name == iface.name)
        {
            if !iface.ip_addr.is_empty() {          
                existing.ip_addr = iface.ip_addr;
            }
            if !iface.mac.is_empty() {
                existing.mac = iface.mac;
            }
        } else {
            merged_result.push(iface);
        }
    }
    // Remove any interfaces that have neither IP nor MAC
    merged_result.retain(|i| !i.ip_addr.is_empty() && !i.mac.is_empty());

    Ok(merged_result)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::log;

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
