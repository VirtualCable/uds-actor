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

pub trait Operations: Send + Sync {
    fn check_permissions(&self) -> anyhow::Result<bool>;

    fn get_computer_name(&self) -> anyhow::Result<String>;

    fn get_domain_name(&self) -> anyhow::Result<Option<String>>;

    fn rename_computer(&self, new_name: &str) -> anyhow::Result<()>;

    fn join_domain(
        &self,
        domain: &str,
        ou: Option<&str>,
        account: &str,
        password: &str,
        execute_in_one_step: bool,
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
pub use crate::windows::operations::new_operations as new_operations;

#[cfg(target_family = "unix")]
pub use crate::unix::operations_linux::new_operations as new_operations;
