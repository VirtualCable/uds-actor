// Fake api to test run function
use crate::rest::{api::ClientRest, types::LoginResponse};
use shared::{
    actions::Actions,
    operations::{NetworkInterface, Operations},
    sync::event::{Event, EventLike},
};
use std::sync::{Arc, RwLock};

#[derive(Clone, Default)]
pub struct Calls {
    inner: Arc<RwLock<Vec<String>>>,
}

impl Calls {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub fn push<S: Into<String>>(&self, call: S) {
        self.inner.write().unwrap().push(call.into());
    }

    pub fn contains_prefix(&self, prefix: &str) -> bool {
        self.inner
            .read()
            .unwrap()
            .iter()
            .any(|c| c.starts_with(prefix))
    }

    pub fn assert_called(&self, prefix: &str) {
        shared::log::info!("Asserting call with prefix: {}", prefix);
        assert!(
            self.contains_prefix(prefix),
            "Expected call starting with '{}', but not found",
            prefix
        );
    }

    pub fn assert_not_called(&self, prefix: &str) {
        shared::log::info!("Asserting NOT called with prefix: {}", prefix);
        assert!(
            !self.contains_prefix(prefix),
            "Did not expect call starting with '{}', but found",
            prefix
        );
    }

    pub fn dump(&self) -> Vec<String> {
        self.inner.read().unwrap().clone()
    }
}

#[derive(Clone)]
struct FakeSessionManager {
    event: Event,
    calls: Calls,
}

impl FakeSessionManager {
    fn new(calls: Calls) -> Self {
        Self {
            event: Event::new(),
            calls,
        }
    }
}

#[async_trait::async_trait]
impl crate::session::SessionManagement for FakeSessionManager {
    async fn wait(&self) {
        self.calls.push("session::wait()");
        self.event.wait_async().await;
    }

    async fn is_running(&self) -> bool {
        self.calls.push("session::is_running()");
        !self.event.is_set()
    }

    async fn stop(&self) {
        self.calls.push("session::stop()");
        self.event.signal();
    }

    async fn wait_timeout(&self, timeout: std::time::Duration) -> bool {
        self.calls
            .push(format!("session::wait_timeout({:?})", timeout));
        let ev = self.event.clone();
        tokio::task::spawn_blocking(move || ev.wait_timeout(timeout))
            .await
            .unwrap()
    }
}

pub struct FakeApi {
    calls: Calls,
}

impl FakeApi {
    fn new(calls: Calls) -> Self {
        Self { calls }
    }
}

#[async_trait::async_trait]
impl ClientRest for FakeApi {
    async fn register(&mut self, _callback_url: &str) -> anyhow::Result<()> {
        self.calls.push("api::register()");
        Ok(())
    }
    async fn unregister(&mut self) -> anyhow::Result<()> {
        self.calls.push("api::unregister()");
        Ok(())
    }
    async fn login(
        &mut self,
        username: &str,
        session_type: Option<&str>,
    ) -> anyhow::Result<LoginResponse> {
        self.calls
            .push(format!("api::login({},{:?})", username, session_type));
        Ok(LoginResponse {
            ip: "127.0.0.1".into(),
            hostname: "localhost".into(),
            deadline: Some(10000),
            max_idle: Some(350),
            session_id: "sessid".into(),
        })
    }
    async fn logout(&self, reason: Option<&str>) -> anyhow::Result<()> {
        self.calls.push(format!("api::logout({:?})", reason));
        Ok(())
    }
    async fn ping(&self) -> anyhow::Result<bool> {
        self.calls.push("api::ping()");
        Ok(true)
    }
}

pub struct FakeActions {
    calls: Calls,
}

impl FakeActions {
    fn new(calls: Calls) -> Self {
        Self { calls }
    }
}

#[async_trait::async_trait]
impl Actions for FakeActions {
    async fn screenshot(&self) -> anyhow::Result<Vec<u8>> {
        self.calls.push("actions::screenshot()");
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
        self.calls.push(format!("actions::run_script({})", script));
        Ok(format!("Executed: {}", script))
    }
    async fn notify_user(&self, message: &str, _gui: shared::gui::GuiHandle) -> anyhow::Result<()> {
        self.calls
            .push(format!("actions::notify_user({:?})", message));
        Ok(())
    }
}

pub struct FakeOperations {
    calls: Calls,
}

impl FakeOperations {
    fn new(calls: Calls) -> Self {
        Self { calls }
    }
}

impl Operations for FakeOperations {
    fn check_permissions(&self) -> anyhow::Result<bool> {
        self.calls.push("operations::check_permissions()");
        Ok(true)
    }

    fn get_computer_name(&self) -> anyhow::Result<String> {
        self.calls.push("operations::get_computer_name()");
        Ok("FakeComputer".into())
    }

    fn get_domain_name(&self) -> anyhow::Result<Option<String>> {
        self.calls.push("operations::get_domain_name()");
        Ok(Some("FakeDomain".into()))
    }

    fn rename_computer(&self, new_name: &str) -> anyhow::Result<()> {
        self.calls
            .push(format!("operations::rename_computer({})", new_name));
        shared::log::info!("Renaming computer to {}", new_name);
        Ok(())
    }

    fn join_domain(&self, options: &shared::operations::JoinDomainOptions) -> anyhow::Result<()> {
        self.calls
            .push(format!("operations::join_domain({:?})", options));
        shared::log::info!("Joining domain: {:?}", options);
        Ok(())
    }

    fn change_user_password(
        &self,
        user: &str,
        old_password: &str,
        new_password: &str,
    ) -> anyhow::Result<()> {
        self.calls.push(format!(
            "operations::change_user_password({},{},{})",
            user, old_password, new_password
        ));
        shared::log::info!(
            "Changing password for user: {}, old: {}, new: {}",
            user,
            old_password,
            new_password
        );
        Ok(())
    }

    fn get_os_version(&self) -> anyhow::Result<String> {
        self.calls.push("operations::get_os_version()");
        Ok("Linux Debian Moscarda Edition 36.11.32".into())
    }

    /// Reboot the machine. `flags` is an optional platform-specific bitmask
    /// represented as `u32` here; the platform implementation must convert it
    /// to the platform-specific flags type.
    fn reboot(&self, flags: Option<u32>) -> anyhow::Result<()> {
        self.calls.push(format!("operations::reboot({:?})", flags));
        shared::log::info!("Rebooting with flags: {:?}", flags);
        Ok(())
    }

    fn logoff(&self) -> anyhow::Result<()> {
        self.calls.push("operations::logoff()");
        Ok(())
    }

    fn init_idle_timer(&self) -> anyhow::Result<()> {
        self.calls.push("operations::init_idle_timer()");
        Ok(())
    }

    fn get_network_info(&self) -> anyhow::Result<Vec<NetworkInterface>> {
        self.calls.push("operations::get_network_info()");
        Ok(vec![NetworkInterface {
            name: "eth0".into(),
            ip_addr: "192.168.1.100".into(),
            mac: "00:1A:2B:3C:4D:5E".into(),
        }])
    }

    fn get_idle_duration(&self) -> anyhow::Result<std::time::Duration> {
        self.calls.push("operations::get_idle_duration()");
        Ok(std::time::Duration::from_secs(600))
    }

    fn get_current_user(&self) -> anyhow::Result<String> {
        self.calls.push("operations::get_current_user()");
        Ok("FakeUser".into())
    }

    fn get_session_type(&self) -> anyhow::Result<String> {
        self.calls.push("operations::get_session_type()");
        Ok("Interactive".into())
    }

    fn force_time_sync(&self) -> anyhow::Result<()> {
        self.calls.push("operations::force_time_sync()");
        Ok(())
    }

    fn protect_file_for_owner_only(&self, _path: &str) -> anyhow::Result<()> {
        self.calls.push(format!(
            "operations::protect_file_for_owner_only({})",
            _path
        ));
        Ok(())
    }
}

pub async fn create_platform(
    manager: Option<std::sync::Arc<dyn crate::session::SessionManagement>>,
    operations: Option<std::sync::Arc<dyn shared::operations::Operations>>,
    api: Option<std::sync::Arc<tokio::sync::RwLock<dyn ClientRest>>>,
    actions: Option<std::sync::Arc<dyn Actions>>,
) -> (crate::platform::Platform, Calls) {
    let calls: Calls = Calls::new();
    let manager =
        manager.unwrap_or_else(|| std::sync::Arc::new(FakeSessionManager::new(calls.clone())));
    let operations =
        operations.unwrap_or_else(|| std::sync::Arc::new(FakeOperations::new(calls.clone())));
    let api = api.unwrap_or_else(|| {
        std::sync::Arc::new(tokio::sync::RwLock::new(FakeApi::new(calls.clone())))
    });
    let actions = actions.unwrap_or_else(|| std::sync::Arc::new(FakeActions::new(calls.clone())));
    (
        crate::platform::Platform::new_with_params(
            Some(manager),
            Some(api),
            Some(operations),
            Some(actions),
        ),
        calls,
    )
}
