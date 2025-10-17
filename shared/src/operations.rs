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

// Shared operations trait for platform-specific implementations.
// This file defines a platform-agnostic trait with the public methods
// implemented for Windows in `shared::windows::operations`.
//
// NOTE: I use primitive types for platform-specific flags (e.g. reboot flags
// are represented as `Option<u32>`) to keep the trait cross-platform.
// The Windows implementation will convert those into the appropriate
// Windows-specific types.
use anyhow::Result;

use crate::log;

// Struct for a network interface information
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    pub name: String,
    pub ip_addr: String,
    pub mac: String,
}

impl NetworkInterface {
    /// Check if this interface's IP is inside the given subnet (IPv4 or IPv6).
    pub fn in_subnet(&self, subnet: Option<&str>) -> bool {
        // If no subnet provided, always valid
        let Some(subnet_str) = subnet else {
            return true;
        };

        // If empty, also always valid
        if subnet_str.trim().is_empty() {
            return true;
        }

        // Try to parse subnet
        let Ok(net) = subnet_str.parse::<ipnetwork::IpNetwork>() else {
            return true; // if subnet invalid, treat as "no filter"
        };

        // Try to parse interface IP
        match self.ip_addr.parse::<std::net::IpAddr>() {
            Ok(addr) => net.contains(addr),
            Err(_) => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct JoinDomainOptions {
    pub domain: String,
    pub account: String,
    pub password: String,
    pub ou: Option<String>,
    // Additional options from custom data
    // These are optional and can be set to None if not provided
    pub client_software: Option<String>,
    pub server_software: Option<String>,
    pub membership_software: Option<String>,
    pub ssl: Option<bool>,
    pub automatic_id_mapping: Option<bool>,
}

pub trait Operations: Send + Sync {
    /// Check if the current user has the necessary permissions to perform administrative tasks.
    fn check_permissions(&self) -> Result<()>;

    /// Get the computer name.
    /// Returns the hostname of the computer.
    fn get_computer_name(&self) -> Result<String>;

    /// Get the domain name the computer is joined to.
    /// Returns `Ok(None)` if the computer is not joined to any domain.
    fn get_domain_name(&self) -> Result<Option<String>>;

    /// Renames the computer to `new_name`.
    /// This may require a reboot to take effect.
    fn rename_computer(&self, new_name: &str) -> Result<()>;

    /// Joins the computer to a domain with the given options.
    /// The `options` struct contains all necessary information for joining the domain.
    fn join_domain(&self, options: &JoinDomainOptions) -> Result<()>;

    /// Change the password for a user.
    /// This may require the old password, depending on the platform and user privileges.
    fn change_user_password(
        &self,
        user: &str,
        old_password: &str,
        new_password: &str,
    ) -> Result<()>;

    fn get_os_version(&self) -> Result<String>;

    /// Reboot the machine. `flags` is an optional platform-specific bitmask
    /// represented as `u32` here; the platform implementation must convert it
    /// to the platform-specific flags type.
    fn reboot(&self, flags: Option<u32>) -> Result<()>;

    /// Log off the current user.
    fn logoff(&self) -> Result<()>;

    // Initializes the idle timer mechanism, if required by the platform.
    // This should be called once during startup.
    fn init_idle_timer(&self, min_required: u64) -> Result<()>;

    /// Get information about the network interfaces on the machine.
    /// This should return a list of all network interfaces, including their IP and MAC addresses.
    /// Excludes loopback and link-local addresses.
    fn get_network_info(&self) -> Result<Vec<NetworkInterface>>;

    // Implicitly, get first interface (if any)
    fn get_first_network_interface(&self) -> Result<NetworkInterface> {
        let ifaces = self.get_network_info()?;
        ifaces
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No network interfaces found on this machine"))
    }

    /// Get the duration the system has been idle (no user input) as a `Duration`.
    /// The definition of "idle" in our case is the time since the last user interaction.
    fn get_idle_duration(&self) -> Result<std::time::Duration>;

    /// Get the current user logged into the system.
    fn get_current_user(&self) -> Result<String>;

    // Get the type of session (e.g., "console", "rdp", etc.)
    fn get_session_type(&self) -> Result<String>;

    /// Force a time synchronization with the time server.
    fn force_time_sync(&self) -> Result<()>;

    /// Protect a file so that only the owner can read/write it.
    /// This is useful for configuration files containing sensitive information.
    /// On Unix, this typically sets permissions to 600. On Windows, it modifies the ACLs.
    fn protect_file_for_owner_only(&self, path: &str) -> Result<()>;

    // Make whatever is is needed to allow the user to connect via RDP
    // This may include enabling RDP, configuring firewall, etc.
    // On windows, basically ensures that the user is in the "Remote Desktop Users" group
    // Linux and macOs, does nothing right now
    fn ensure_user_can_rdp(&self, user: &str) -> Result<()>;

    // This specifically checks if there is any installation in progress (like Windows Update)
    // On unix, this will always return false
    fn is_some_installation_in_progress(&self) -> Result<bool>;

    // Get an screenshot of the current desktop
    fn get_screenshot(&self) -> Result<Vec<u8>> {
        log::info!("Screenshot requested (stub)");
        // TODO: Implement screenshot on Unix. Return 1x1 transparent PNG for now.
        const PNG_1X1_TRANSPARENT: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        Ok(PNG_1X1_TRANSPARENT.to_vec())
    }
}

// Re-export the Windows concrete implementation when building for Windows.
//
// NOTE: I export it as `WindowsOperationsImpl` to avoid name conflicts with
// the trait itself. If you prefer the concrete type to be re-exported as
// `Operations`, I can change it (but that will shadow the trait name in this
// module).
#[cfg(target_os = "windows")]
pub use crate::windows::operations::new_operations;

#[cfg(target_family = "unix")]
pub use crate::unix::linux::new_operations;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_interface_in_subnet() {
        let iface = NetworkInterface {
            name: "eth0".to_string(),
            mac: "00:11:22:33:44:55".to_string(),
            ip_addr: "192.168.1.10".to_string(),
        };
        assert!(iface.in_subnet(Some("192.168.1.0/24")));
        assert!(!iface.in_subnet(Some("192.168.2.0/24")));
    }

    #[test]
    fn test_multiple_interfaces_in_subnet() {
        let ifaces = vec![
            NetworkInterface {
                name: "eth0".to_string(),
                mac: "00:11:22:33:44:55".to_string(),
                ip_addr: "192.168.1.10".to_string(),
            },
            NetworkInterface {
                name: "eth1".to_string(),
                mac: "00:11:22:33:44:56".to_string(),
                ip_addr: "192.168.1.11".to_string(),
            },
            NetworkInterface {
                name: "eth2".to_string(),
                mac: "00:11:22:33:44:57".to_string(),
                ip_addr: "192.168.1.12".to_string(),
            },
            // Not in subnet
            NetworkInterface {
                name: "eth3".to_string(),
                mac: "00:11:22:33:44:58".to_string(),
                ip_addr: "192.168.2.10".to_string(),
            },
        ];
        let in_subnet: Vec<_> = ifaces
            .iter()
            .filter(|iface| iface.in_subnet(Some("192.168.1.0/24")))
            .collect();
        assert_eq!(in_subnet.len(), 3);
        let not_in_subnet: Vec<_> = ifaces
            .iter()
            .filter(|iface| !iface.in_subnet(Some("192.168.1.0/24")))
            .collect();
        assert_eq!(not_in_subnet.len(), 1);
    }
}
