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

// Struct for a network interface information
#[derive(Debug, Clone)]
pub struct NetworkInterfaceInfo {
    pub name: String,
    pub ip_address: String,
    pub mac: String,
}

#[derive(Debug, Clone)]
pub struct JoinDomainOptions {
    pub domain: String,
    pub account: String,
    pub password: String,
    pub ou: Option<String>,
    pub execute_in_one_step: Option<bool>,
    // Additional options from custom data
    // These are optional and can be set to None if not provided
    pub client_software: Option<String>,
    pub server_software: Option<String>,
    pub membership_software: Option<String>,
    pub ssl: Option<bool>,
    pub automatic_id_mapping: Option<bool>,
}

pub trait Operations: Send + Sync {
    fn check_permissions(&self) -> anyhow::Result<bool>;

    fn get_computer_name(&self) -> anyhow::Result<String>;

    fn get_domain_name(&self) -> anyhow::Result<Option<String>>;

    fn rename_computer(&self, new_name: &str) -> anyhow::Result<()>;

    fn join_domain(
        &self,
        options: &JoinDomainOptions,
    ) -> anyhow::Result<()>;

    fn change_user_password(
        &self,
        user: &str,
        old_password: &str,
        new_password: &str,
    ) -> anyhow::Result<()>;

    fn get_os_version(&self) -> anyhow::Result<String>;

    /// Reboot the machine. `flags` is an optional platform-specific bitmask
    /// represented as `u32` here; the platform implementation must convert it
    /// to the platform-specific flags type.
    fn reboot(&self, flags: Option<u32>) -> anyhow::Result<()>;

    fn logoff(&self) -> anyhow::Result<()>;

    fn init_idle_timer(&self) -> anyhow::Result<()>;

    fn get_network_info(&self) -> anyhow::Result<Vec<NetworkInterfaceInfo>>;

    fn get_idle_duration(&self) -> anyhow::Result<std::time::Duration>;

    fn get_current_user(&self) -> anyhow::Result<String>;

    fn get_session_type(&self) -> anyhow::Result<String>;

    fn force_time_sync(&self) -> anyhow::Result<()>;

    fn protect_file_for_owner_only(&self, path: &str) -> anyhow::Result<()>;
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
pub use crate::unix::operations_linux::new_operations;
