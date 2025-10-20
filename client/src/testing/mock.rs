use anyhow::Result;

use shared::sync::OnceSignal;
use shared::ws::client::websocket_client_tasks;

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

pub async fn mock_platform(
    manager: Option<std::sync::Arc<dyn crate::session::SessionManagement>>,
    operations: Option<std::sync::Arc<dyn shared::operations::Operations>>,
    port: u16,
) -> (crate::platform::Platform, Calls) {
    let calls: Calls = Calls::new();
    let manager =
        manager.unwrap_or_else(|| std::sync::Arc::new(SessionManagerMock::new(calls.clone())));
    let operations =
        operations.unwrap_or_else(|| std::sync::Arc::new(OperationsMock::new(calls.clone())));
    let ws_client = websocket_client_tasks(port, 32).await;

    (
        crate::platform::Platform::new_with_params(
            Some(manager),
            Some(operations),
            Some(ws_client),
            port,
        )
        .await,
        calls,
    )
}
