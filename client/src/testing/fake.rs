// Fake api to test run function
use crate::rest::{api::ClientRest, types::LoginResponse};
use shared::{
    actions::Actions,
    sync::event::{Event, EventLike},
};

#[derive(Clone)]
struct FakeSessionManager {
    event: Event,
}

impl Default for FakeSessionManager {
    fn default() -> Self {
        Self {
            event: Event::new(),
        }
    }
}

#[async_trait::async_trait]
impl crate::session::SessionManagement for FakeSessionManager {
    async fn wait(&self) {
        self.event.wait_async().await;
    }

    async fn is_running(&self) -> bool {
        !self.event.is_set()
    }

    async fn stop(&self) {
        self.event.signal();
    }

    async fn wait_timeout(&self, timeout: std::time::Duration) -> bool {
        let ev = self.event.clone();
        tokio::task::spawn_blocking(move || ev.wait_timeout(timeout))
            .await
            .unwrap()
    }
}

#[derive(Default)]
pub struct FakeApi {}

#[async_trait::async_trait]
impl ClientRest for FakeApi {
    async fn register(&mut self, _callback_url: &str) -> Result<(), reqwest::Error> {
        Ok(())
    }
    async fn unregister(&mut self) -> Result<(), reqwest::Error> {
        Ok(())
    }
    async fn login(
        &mut self,
        _username: &str,
        _session_type: Option<&str>,
    ) -> Result<LoginResponse, reqwest::Error> {
        Ok(LoginResponse {
            ip: "127.0.0.1".into(),
            hostname: "localhost".into(),
            deadline: Some(10000),
            max_idle: Some(350),
            session_id: "sessid".into(),
        })
    }
    async fn logout(
        &self,
    ) -> Result<(), reqwest::Error> {
        Ok(())
    }
    async fn ping(&self) -> Result<bool, reqwest::Error> {
        Ok(true)
    }
}

#[derive(Default)]
pub struct FakeActions {
}

#[async_trait::async_trait]
impl Actions for FakeActions {
    async fn logoff(&self) -> anyhow::Result<()> {
        Ok(())
    }
    async fn screenshot(&self) -> anyhow::Result<Vec<u8>> {
        const PNG_1X1_TRANSPARENT: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        Ok(PNG_1X1_TRANSPARENT.to_vec())
    }
    async fn run_script(&self, script: &str) -> anyhow::Result<String> {
        Ok(format!("Executed: {}", script))
    }
    async fn notify_user(&self, _message: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Default)]
pub struct FakeOperations {}

impl shared::operations::Operations for FakeOperations {
    fn check_permissions(&self) -> anyhow::Result<bool> {
        Ok(true)
    }

    fn get_computer_name(&self) -> anyhow::Result<String> {
        Ok("FakeComputer".into())
    }

    fn get_domain_name(&self) -> anyhow::Result<Option<String>> {
        Ok(Some("FakeDomain".into()))
    }

    fn rename_computer(&self, new_name: &str) -> anyhow::Result<()> {
        println!("Renaming computer to {}", new_name);
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
        println!(
            "Joining domain: {}, ou: {:?}, account: {}, password: {}, one_step: {}",
            domain, ou, account, password, execute_in_one_step
        );
        Ok(())
    }

    fn change_user_password(
        &self,
        user: &str,
        old_password: &str,
        new_password: &str,
    ) -> anyhow::Result<()> {
        println!(
            "Changing password for user: {}, old: {}, new: {}",
            user, old_password, new_password
        );
        Ok(())
    }

    fn get_windows_version(&self) -> anyhow::Result<(u32, u32, u32, u32, String)> {
        Ok((10, 0, 19044, 0, "Windows 10".into()))
    }

    fn get_os_version(&self) -> anyhow::Result<String> {
        Ok("Windows 10".into())
    }

    /// Reboot the machine. `flags` is an optional platform-specific bitmask
    /// represented as `u32` here; the platform implementation must convert it
    /// to the platform-specific flags type.
    fn reboot(&self, flags: Option<u32>) -> anyhow::Result<()> {
        println!("Rebooting with flags: {:?}", flags);
        Ok(())
    }

    fn logoff(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn init_idle_timer(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn get_network_info(&self) -> anyhow::Result<Vec<(String, String, String)>> {
        Ok(vec![(
            "eth0".into(),
            "192.168.1.100".into(),
            "255.255.255.0".into(),
        )])
    }

    fn get_idle_duration(&self) -> anyhow::Result<std::time::Duration> {
        Ok(std::time::Duration::from_secs(300))
    }

    fn get_current_user(&self) -> anyhow::Result<String> {
        Ok("FakeUser".into())
    }

    fn get_session_type(&self) -> anyhow::Result<String> {
        Ok("Interactive".into())
    }

    fn force_time_sync(&self) -> anyhow::Result<()> {
        Ok(())
    }

    fn protect_file_for_owner_only(&self, _path: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

pub async fn create_platform(
    manager: Option<std::sync::Arc<dyn crate::session::SessionManagement>>,
    operations: Option<std::sync::Arc<dyn shared::operations::Operations>>,
    api: Option<std::sync::Arc<tokio::sync::RwLock<dyn ClientRest>>>,
    actions: Option<std::sync::Arc<dyn Actions>>,
) -> crate::platform::Platform {
    let manager = manager.unwrap_or_else(|| std::sync::Arc::new(FakeSessionManager::default()));
    let operations = operations.unwrap_or_else(|| std::sync::Arc::new(FakeOperations::default()));
    let api = api.unwrap_or_else(|| std::sync::Arc::new(tokio::sync::RwLock::new(FakeApi {})));
    let actions = actions.unwrap_or_else(|| std::sync::Arc::new(FakeActions::default()));
    crate::platform::Platform::new_with_params(
        Some(manager),
        Some(api),
        Some(operations),
        Some(actions),
    )
}
