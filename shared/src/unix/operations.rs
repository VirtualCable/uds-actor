// Minimal UnixOperations stub implementation.
// Each method logs its parameters and returns a safe default value.

use anyhow::Context;
use crate::log;

pub struct UnixOperations;

impl UnixOperations {
    pub fn new() -> Self {
        Self {}
    }
}

impl crate::operations::Operations for UnixOperations {
    fn check_permissions(&self) -> anyhow::Result<bool> {
        log::debug!("UnixOperations::check_permissions called");
        Ok(false)
    }

    fn get_computer_name(&self) -> anyhow::Result<String> {
        log::debug!("UnixOperations::get_computer_name called");
        Ok(String::new())
    }

    fn get_domain_name(&self) -> anyhow::Result<Option<String>> {
        log::debug!("UnixOperations::get_domain_name called");
        Ok(None)
    }

    fn rename_computer(&self, new_name: &str) -> anyhow::Result<()> {
        log::debug!("UnixOperations::rename_computer called: {}", new_name);
        Ok(())
    }

    fn join_domain(
        &self,
        domain: &str,
        ou: Option<&str>,
        account: &str,
        password: &str,
        execute_in_one_step: bool,
    ) -> anyhow::Result<()> {
        log::debug!(
            "UnixOperations::join_domain called: domain={} ou={:?} account={} execute_in_one_step={}",
            domain,
            ou,
            account,
            execute_in_one_step
        );
        Ok(())
    }

    fn change_user_password(
        &self,
        user: &str,
        old_password: &str,
        new_password: &str,
    ) -> anyhow::Result<()> {
        log::debug!(
            "UnixOperations::change_user_password called: user={}",
            user
        );
        Ok(())
    }

    fn get_windows_version(&self) -> anyhow::Result<(u32, u32, u32, u32, String)> {
        log::debug!("UnixOperations::get_windows_version called");
        Ok((0, 0, 0, 0, String::new()))
    }

    fn get_os_version(&self) -> anyhow::Result<String> {
        log::debug!("UnixOperations::get_os_version called");
        Ok(String::new())
    }

    fn reboot(&self, flags: Option<u32>) -> anyhow::Result<()> {
        log::debug!("UnixOperations::reboot called: {:?}", flags);
        Ok(())
    }

    fn logoff(&self) -> anyhow::Result<()> {
        log::debug!("UnixOperations::logoff called");
        Ok(())
    }

    fn init_idle_timer(&self) -> anyhow::Result<()> {
        log::debug!("UnixOperations::init_idle_timer called");
        Ok(())
    }

    fn get_network_info(&self) -> anyhow::Result<Vec<(String, String, String)>> {
        log::debug!("UnixOperations::get_network_info called");
        Ok(vec![])
    }

    fn get_idle_duration(&self) -> anyhow::Result<f64> {
        log::debug!("UnixOperations::get_idle_duration called");
        Ok(0.0)
    }

    fn get_current_user(&self) -> anyhow::Result<String> {
        log::debug!("UnixOperations::get_current_user called");
        Ok(String::new())
    }

    fn get_session_type(&self) -> anyhow::Result<String> {
        log::debug!("UnixOperations::get_session_type called");
        Ok(String::new())
    }

    fn force_time_sync(&self) -> anyhow::Result<()> {
        log::debug!("UnixOperations::force_time_sync called");
        Ok(())
    }

    fn protect_file_for_owner_only(&self, path: &str) -> anyhow::Result<()> {
        log::debug!("UnixOperations::protect_file_for_owner_only called: {}", path);
        Ok(())
    }
}
