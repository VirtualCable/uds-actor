// Fake api to test run function
use crate::{
    actions::Actions,
    broker::api,
    operations::{NetworkInterface, Operations},
    tls::CertificateInfo,
};
use std::sync::{Arc, RwLock};

use super::test_certs;

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
        crate::log::info!("Asserting call with prefix: {}", prefix);
        assert!(
            self.contains_prefix(prefix),
            "Expected call starting with '{}', but not found",
            prefix
        );
    }

    pub fn assert_not_called(&self, prefix: &str) {
        crate::log::info!("Asserting NOT called with prefix: {}", prefix);
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

pub struct DummyActions {
    calls: Calls,
}

impl DummyActions {
    pub fn new(calls: Calls) -> Self {
        Self { calls }
    }
}

#[async_trait::async_trait]
impl Actions for DummyActions {
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
    async fn notify_user(&self, message: &str, _gui: crate::gui::GuiHandle) -> anyhow::Result<()> {
        self.calls
            .push(format!("actions::notify_user({:?})", message));
        Ok(())
    }
}

pub struct DummyOperations {
    pub calls: Calls,
}

impl DummyOperations {
    pub fn new(calls: Calls) -> Self {
        Self { calls }
    }
}

impl Default for DummyOperations {
    fn default() -> Self {
        Self {
            calls: Calls::new(),
        }
    }
}

impl Operations for DummyOperations {
    fn check_permissions(&self) -> anyhow::Result<()> {
        self.calls.push("operations::check_permissions()");
        Ok(())
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
        crate::log::info!("Renaming computer to {}", new_name);
        Ok(())
    }

    fn join_domain(&self, options: &crate::operations::JoinDomainOptions) -> anyhow::Result<()> {
        self.calls
            .push(format!("operations::join_domain({:?})", options));
        crate::log::info!("Joining domain: {:?}", options);
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
        crate::log::info!(
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
        crate::log::info!("Rebooting with flags: {:?}", flags);
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
        Ok(vec![
            NetworkInterface {
                name: "eth0".into(),
                ip_addr: "192.168.1.100".into(),
                mac: "00:1A:2B:3C:4D:5E".into(),
            },
            NetworkInterface {
                name: "wlan0".into(),
                ip_addr: "192.168.1.101".into(),
                mac: "00:1A:2B:3C:4D:5F".into(),
            },
            NetworkInterface {
                name: "docker0".into(),
                ip_addr: "169.254.0.1".into(),
                mac: "00:1A:2B:3C:4D:5H".into(),
            },
        ])
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

    fn ensure_user_can_rdp(&self, user: &str) -> anyhow::Result<()> {
        self.calls
            .push(format!("operations::ensure_user_can_rdp({})", user));
        Ok(())
    }
    
    fn is_some_installation_in_progress(&self) -> anyhow::Result<bool> {
        self.calls
            .push("operations::is_some_installation_in_progress()");
        Ok(false)
    }
}

#[derive(Clone)]
pub struct DummyBrokerApi {
    calls: Calls,
    secret: Option<String>,
}

impl DummyBrokerApi {
    pub fn new(calls: Calls) -> Self {
        Self {
            calls,
            secret: None,
        }
    }
}

#[async_trait::async_trait]
impl api::BrokerApi for DummyBrokerApi {
    fn clear_headers(&mut self) {
        self.calls.push("broker_api::clear_headers()");
    }

    fn set_header(&mut self, key: &str, value: &str) {
        self.calls
            .push(format!("broker_api::set_header({}, {})", key, value));
    }

    fn get_secret(&self) -> Result<&str, api::types::RestError> {
        self.calls.push("broker_api::get_secret()");
        self.secret
            .as_deref()
            .ok_or_else(|| api::types::RestError::Other("No secret set".into()))
    }
    async fn enumerate_authenticators(
        &self,
    ) -> Result<Vec<api::types::Authenticator>, api::types::RestError> {
        self.calls.push("broker_api::enumerate_authenticators()");
        Ok(vec![
            api::types::Authenticator {
                id: "auth1".to_string(),
                label: "Auth One".to_string(),
                name: "auth1".to_string(),
                auth_type: "type1".to_string(),
                priority: 1,
                custom: false,
            },
            api::types::Authenticator {
                id: "auth2".to_string(),
                label: "Auth Two".to_string(),
                name: "auth2".to_string(),
                auth_type: "type2".to_string(),
                priority: 2,
                custom: true,
            },
        ])
    }

    async fn api_login(
        &self,
        auth: &str,
        username: &str,
        _password: &str,
    ) -> Result<String, api::types::RestError> {
        self.calls.push(format!(
            "broker_api::api_login({}, {}, ***)",
            auth, username
        ));
        Ok("fake_auth_token".into())
    }

    async fn register(
        &self,
        req: &api::types::RegisterRequest,
    ) -> Result<String, api::types::RestError> {
        self.calls.push(format!("broker_api::register({:?})", req));
        Ok("fake_registration_token".into())
    }
    async fn initialize(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
    ) -> Result<api::types::InitializationResponse, api::types::RestError> {
        self.calls
            .push(format!("broker_api::initialize({:?})", interfaces));
        Ok(api::types::InitializationResponse {
            master_token: Some("fake_master_token".into()),
            token: Some("fake_token".into()),
            unique_id: Some("fake_unique_id".into()),
            os: None,
        })
    }
    async fn ready(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<CertificateInfo, api::types::RestError> {
        self.calls
            .push(format!("broker_api::ready({}, {})", ip, port));
        Ok(test_certs::test_certinfo_with_pass())
    }
    async fn unmanaged_ready(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
        port: u16,
    ) -> Result<CertificateInfo, api::types::RestError> {
        self.calls.push(format!(
            "broker_api::unmanaged_ready({:?}, {})",
            interfaces, port
        ));
        Ok(test_certs::test_certinfo_with_pass())
    }
    async fn notify_new_ip(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<CertificateInfo, api::types::RestError> {
        self.calls
            .push(format!("broker_api::notify_new_ip({}, {})", ip, port));
        Ok(test_certs::test_certinfo())
    }
    async fn login(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
        username: &str,
        session_type: &str,
    ) -> Result<api::types::LoginResponse, api::types::RestError> {
        self.calls.push(format!(
            "broker_api::login({:?}, {}, {})",
            interfaces, username, session_type
        ));
        Ok(api::types::LoginResponse {
            ip: "fake_ip".into(),
            hostname: "fakeHost".into(),
            deadline: Some(3600),
            max_idle: Some(600),
            session_id: Some("fake_session_id".into()),
        })
    }
    async fn logout(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
        username: &str,
        session_type: &str,
        session_id: &str,
    ) -> Result<String, api::types::RestError> {
        self.calls.push(format!(
            "broker_api::logout({:?}, {}, {}, {})",
            interfaces, username, session_type, session_id
        ));
        Ok("Logged out".into())
    }
    async fn log(
        &self,
        level: api::types::LogLevel,
        message: &str,
    ) -> Result<String, api::types::RestError> {
        self.calls
            .push(format!("broker_api::log({:?}, {})", level, message));
        Ok("Log received".into())
    }
    async fn test(&self) -> Result<String, api::types::RestError> {
        self.calls.push("broker_api::test()");
        Ok("Test successful".into())
    }
}
