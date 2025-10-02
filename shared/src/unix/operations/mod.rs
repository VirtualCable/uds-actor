// TODO: finish Unix implementation
use crate::log;

mod renamer;

pub fn new_operations() -> std::sync::Arc<dyn crate::operations::Operations + Send + Sync> {
    std::sync::Arc::new(UnixOperations::new())
}

pub struct UnixOperations;

impl UnixOperations {
    pub fn new() -> Self {
        Self {}
    }

    pub fn get_linux_version(&self) -> Option<String> {
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("ID=") {
                    return Some(line[3..].trim_matches('"').to_string());
                }
            }
        }
        None
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
        renamer::renamer(
            new_name,
            self.get_linux_version().as_deref().unwrap_or("unknown"),
        )
    }

    fn join_domain(
        &self,
        domain: &str,
        ou: Option<&str>,
        account: &str,
        _password: &str,
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
        _old_password: &str,
        _new_password: &str,
    ) -> anyhow::Result<()> {
        log::debug!("UnixOperations::change_user_password called: user={}", user);
        Ok(())
    }

    fn get_os_version(&self) -> anyhow::Result<String> {
        log::debug!("UnixOperations::get_os_version called");
        Ok(self
            .get_linux_version()
            .unwrap_or("generic-linux".to_string()))
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

    fn get_network_info(&self) -> anyhow::Result<Vec<crate::operations::NetworkInterfaceInfo>> {
        log::debug!("UnixOperations::get_network_info called");
        Ok(vec![])
    }

    fn get_idle_duration(&self) -> anyhow::Result<std::time::Duration> {
        log::debug!("UnixOperations::get_idle_duration called");
        Ok(std::time::Duration::new(0, 0))
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
        log::debug!(
            "UnixOperations::protect_file_for_owner_only called: {}",
            path
        );
        Ok(())
    }
}
