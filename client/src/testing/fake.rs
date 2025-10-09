// Fake api to test run function
use crate::rest::{api::ClientRest, types::LoginResponse};
use shared::{
    actions::Actions,
    sync::event::{Event, EventLike},
};

use shared::testing::fake::{Calls, FakeActions, FakeOperations};

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
