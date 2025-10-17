use anyhow::Result;

use crate::rest::{api::ClientRest, types::LoginResponse};
use shared::{
    sync::OnceSignal,
};

use shared::testing::mock::{Calls, OperationsMock};

#[derive(Clone)]
struct SessionManagerMock {
    event: OnceSignal,
    calls: Calls,
}

impl SessionManagerMock {
    fn new(calls: Calls) -> Self {
        Self {
            event: OnceSignal::new(),
            calls,
        }
    }
}

#[async_trait::async_trait]
impl crate::session::SessionManagement for SessionManagerMock {
    async fn wait(&self) {
        self.calls.push("session::wait()");
        self.event.wait().await;
    }

    async fn is_running(&self) -> bool {
        self.calls.push("session::is_running()");
        !self.event.is_set()
    }

    async fn stop(&self) {
        self.calls.push("session::stop()");
        self.event.set();
    }

    async fn wait_timeout(&self, timeout: std::time::Duration) -> Result<()> {
        self.calls
            .push(format!("session::wait_timeout({:?})", timeout));
        let ev = self.event.clone();
        ev.wait_timeout(timeout).await
    }
}

pub struct ApiMock {
    calls: Calls,
}

impl ApiMock {
    fn new(calls: Calls) -> Self {
        Self { calls }
    }
}

#[async_trait::async_trait]
impl ClientRest for ApiMock {
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
}

pub async fn mock_platform(
    manager: Option<std::sync::Arc<dyn crate::session::SessionManagement>>,
    operations: Option<std::sync::Arc<dyn shared::operations::Operations>>,
    api: Option<std::sync::Arc<tokio::sync::RwLock<dyn ClientRest>>>,
) -> (crate::platform::Platform, Calls) {
    let calls: Calls = Calls::new();
    let manager =
        manager.unwrap_or_else(|| std::sync::Arc::new(SessionManagerMock::new(calls.clone())));
    let operations =
        operations.unwrap_or_else(|| std::sync::Arc::new(OperationsMock::new(calls.clone())));
    let api = api.unwrap_or_else(|| {
        std::sync::Arc::new(tokio::sync::RwLock::new(ApiMock::new(calls.clone())))
    });
    (
        crate::platform::Platform::new_with_params(
            Some(manager),
            Some(api),
            Some(operations),
        ).await,
        calls,
    )
}
