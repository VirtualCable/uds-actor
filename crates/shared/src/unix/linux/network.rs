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
use std::os::raw::c_int;
use std::ptr;

use libc::{self};

use anyhow::Result;

use crate::{log, system::NetworkInterface};

// Compile-time ABI guard: struct ifreq must stay the kernel's size on LP64
// targets (40 bytes). If the libc export ever drifts, fail the build instead of
// silently corrupting stack around the ioctl.
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<libc::ifreq>() == 40);

/// Returns iterator (Vec) of InterfaceInfo for “valid” interfaces.
pub fn get_network_info() -> Result<Vec<NetworkInterface>> {
    let names = list_interfaces()?;

    let mut out = Vec::new();
    for ifname in names {
        let ip = get_ipv4_addr(&ifname);
        let mac = get_mac_addr(&ifname);
        if let (Some(ip_address), Some(mac)) = (ip, mac)
            && mac != "00:00:00:00:00:00"
            && !ip_address.starts_with("169.254")
        {
            out.push(NetworkInterface {
                name: ifname,
                ip_addr: ip_address,
                mac,
            });
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
    let mut space = max_possible * 16;

    // Prepare ifconf
    #[repr(C)]
    struct IfConf {
        ifc_len: c_int,
        ifc_buf: *mut libc::c_void,
    }

    // Grow-and-retry: SIOCGIFCONF silently truncates when the buffer is too
    // small, and the truncation is otherwise undetectable (the kernel just
    // reports what it wrote). While the buffer fills to within one entry of
    // capacity, double it and re-probe.
    let names_buf = loop {
        let mut buf = vec![0u8; space];
        let mut ifc = IfConf {
            ifc_len: space as c_int,
            ifc_buf: buf.as_mut_ptr() as *mut libc::c_void,
        };
        let res = unsafe { libc::ioctl(fd, libc::SIOCGIFCONF, &mut ifc) }; // SIOCGIFCONF
        if res < 0 {
            unsafe { libc::close(fd) };
            return Err(io::Error::last_os_error());
        }
        let written = ifc.ifc_len.max(0) as usize;
        if written + 40 <= space || space >= (1 << 20) {
            if written + 40 > space {
                log::warn!(
                    "SIOCGIFCONF truncated at {} bytes; interface list may be incomplete",
                    space
                );
            }
            break (buf, written);
        }
        space *= 2; // grow and re-probe
    };
    let (names_buf, outbytes) = names_buf;
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
        if nul_pos > 0
            && let Ok(name) = std::str::from_utf8(&chunk[..nul_pos])
        {
            names.push(name.to_string());
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

    // Use the kernel-ABI-sized struct from libc (ifr_name[16] + the full ifru
    // union). The previous hand-rolled 32-byte stand-in made the kernel's
    // 40-byte struct ifreq copy run 8 bytes past the local allocation.
    let mut ifr: libc::ifreq = unsafe { mem::zeroed() };
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
    let res = unsafe {
        libc::ioctl(
            sock,
            0x8915,
            &mut ifr as *mut libc::ifreq as *mut libc::c_void,
        )
    }; // SIOCGIFADDR
    if res < 0 {
        unsafe { libc::close(sock) };
        return None;
    }

    // Extract IPv4 from sa_data (standard is bytes 2..6: first 2 are address-family padding).
    let sa_data = unsafe { ifr.ifr_ifru.ifru_addr.sa_data };
    let octets = [sa_data[2] as u8, sa_data[3] as u8, sa_data[4] as u8, sa_data[5] as u8];
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

    // Kernel-ABI struct ifreq (see get_ipv4_addr): the MAC is in the union's
    // ifru_hwaddr.sa_data[0..6] for ARPHRD_ETHER.
    let mut ifr: libc::ifreq = unsafe { mem::zeroed() };

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
    let res = unsafe {
        libc::ioctl(
            sock,
            0x8927,
            &mut ifr as *mut libc::ifreq as *mut libc::c_void,
        )
    }; // SIOCGIFHWADDR
    if res < 0 {
        unsafe { libc::close(sock) };
        return None;
    }

    // Extract MAC from sa_data[0..6]
    let sa_data = unsafe { ifr.ifr_ifru.ifru_hwaddr.sa_data };
    let mac = format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        sa_data[0] as u8,
        sa_data[1] as u8,
        sa_data[2] as u8,
        sa_data[3] as u8,
        sa_data[4] as u8,
        sa_data[5] as u8
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
