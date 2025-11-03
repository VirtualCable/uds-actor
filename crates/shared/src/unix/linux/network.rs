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

use crate::{log, system::NetworkInterface};

/// Returns iterator (Vec) of InterfaceInfo for “valid” interfaces.
pub fn get_network_info() -> Result<Vec<NetworkInterface>> {
    let names = list_interfaces()?;

    let mut out = Vec::new();
    for ifname in names {
        let ip = get_ipv4_addr(&ifname);
        let mac = get_mac_addr(&ifname);
        if let (Some(ip_address), Some(mac)) = (ip, mac) {
            if mac != "00:00:00:00:00:00" && !ip_address.starts_with("169.254") {
                out.push(NetworkInterface {
                    name: ifname,
                    ip_addr: ip_address,
                    mac,
                });
            }
        }
    }
    Ok(out)
}

/// List interface names using SIOCGIFCONF. Mirrors Python approach with arch-specific stride.
fn list_interfaces() -> io::Result<Vec<String>> {
    let sock = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if sock < 0 {
        return Err(io::Error::last_os_error());
    }
    let fd = sock;

    // Similar to Python: allocate a buffer and let SIOCGIFCONF fill it.
    // We will decode names using stride that depends on arch.
    let max_possible = 128; // arbitrary, raise if needed
    let space = max_possible * 16;
    let mut names_buf = vec![0u8; space];

    // Prepare ifconf
    #[repr(C)]
    struct IfConf {
        ifc_len: c_int,
        ifc_buf: *mut libc::c_void,
    }
    let mut ifc = IfConf {
        ifc_len: space as c_int,
        ifc_buf: names_buf.as_mut_ptr() as *mut libc::c_void,
    };

    let res = unsafe { libc::ioctl(fd, libc::SIOCGIFCONF, &mut ifc) }; // SIOCGIFCONF
    if res < 0 {
        unsafe { libc::close(fd) };
        return Err(io::Error::last_os_error());
    }
    // outbytes
    let outbytes = ifc.ifc_len as usize;
    let namestr = &names_buf[..outbytes];

    let (offset, length) = if cfg!(target_pointer_width = "32") {
        (32usize, 32usize) // 32-bit
    } else {
        (16usize, 40usize) // 64-bit (x86_64, aarch64, etc.)
    };

    // Parse names out of the buffer
    let mut names = Vec::new();
    let mut i = 0usize;
    while i + length <= outbytes {
        let chunk = &namestr[i..i + offset];
        // split at first NUL
        let nul_pos = chunk.iter().position(|&b| b == 0).unwrap_or(chunk.len());
        if nul_pos > 0 {
            if let Ok(name) = std::str::from_utf8(&chunk[..nul_pos]) {
                names.push(name.to_string());
            }
        }
        i += length;
    }

    unsafe { libc::close(fd) };
    log::debug!("Found interfaces: {:?}", names);
    Ok(names)
}

/// Get IPv4 address for interface via SIOCGIFADDR.
fn get_ipv4_addr(ifname: &str) -> Option<String> {
    let sock = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if sock < 0 {
        return None;
    }

    // Build ifreq
    #[repr(C)]
    struct IfReq {
        ifr_name: [c_char; libc::IFNAMSIZ],
        ifr_addr: sockaddr, // we'll only read sa_data
    }
    let mut ifr: IfReq = unsafe { mem::zeroed() };
    // copy name
    let name_bytes = ifname.as_bytes();
    let len = name_bytes.len().min(libc::IFNAMSIZ - 1);
    unsafe {
        ptr::copy_nonoverlapping(
            name_bytes.as_ptr(),
            ifr.ifr_name.as_mut_ptr() as *mut u8,
            len,
        );
        ifr.ifr_name[len] = 0;
    }

    // ioctl SIOCGIFADDR
    let res = unsafe { libc::ioctl(sock, 0x8915, &mut ifr) }; // SIOCGIFADDR
    if res < 0 {
        unsafe { libc::close(sock) };
        return None;
    }

    // Extract IPv4 from sa_data (offset differs; standard is bytes 2..6).
    let data = unsafe {
        std::slice::from_raw_parts(
            &ifr.ifr_addr.sa_data as *const _ as *const u8,
            mem::size_of_val(&ifr.ifr_addr.sa_data),
        )
    };
    if data.len() < 6 {
        unsafe { libc::close(sock) };
        return None;
    }
    // sa_data: first 2 bytes are port family padding, next 4 are IPv4
    let octets = [data[2], data[3], data[4], data[5]];
    let ip = Ipv4Addr::from(octets).to_string();

    unsafe { libc::close(sock) };
    log::debug!("Found IPv4 address for {}: {}", ifname, ip);
    Some(ip)
}

/// Get MAC address via SIOCGIFHWADDR.
fn get_mac_addr(ifname: &str) -> Option<String> {
    let sock = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if sock < 0 {
        return None;
    }

    // Build ifreq with union field; we only need name and hwaddr bytes.
    #[repr(C)]
    struct IfReqHw {
        ifr_name: [c_char; libc::IFNAMSIZ],
        ifr_hwaddr: sockaddr, // sa_data[0..6] are MAC for ARPHRD_ETHER
    }
    let mut ifr: IfReqHw = unsafe { mem::zeroed() };

    // copy name
    let name_bytes = ifname.as_bytes();
    let len = name_bytes.len().min(libc::IFNAMSIZ - 1);
    unsafe {
        ptr::copy_nonoverlapping(
            name_bytes.as_ptr(),
            ifr.ifr_name.as_mut_ptr() as *mut u8,
            len,
        );
        ifr.ifr_name[len] = 0;
    }

    // ioctl SIOCGIFHWADDR
    let res = unsafe { libc::ioctl(sock, 0x8927, &mut ifr) }; // SIOCGIFHWADDR
    if res < 0 {
        unsafe { libc::close(sock) };
        return None;
    }

    // Extract MAC from sa_data[0..6]
    let data = unsafe {
        std::slice::from_raw_parts(
            &ifr.ifr_hwaddr.sa_data as *const _ as *const u8,
            mem::size_of_val(&ifr.ifr_hwaddr.sa_data),
        )
    };
    if data.len() < 6 {
        unsafe { libc::close(sock) };
        return None;
    }
    let mac = format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        data[0], data[1], data[2], data[3], data[4], data[5]
    );

    unsafe { libc::close(sock) };

    log::debug!("Found MAC address for {}: {}", ifname, mac);
    Some(mac)
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
