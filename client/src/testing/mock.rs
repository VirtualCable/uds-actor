use shared::sync::OnceSignal;
use shared::ws::client::WsClient;

use shared::testing::mock::{Calls, OperationsMock};

use tokio::sync::{broadcast, mpsc};

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
    fn get_stop(&self) -> OnceSignal {
        self.calls.push("session::get_stop()");
        self.event.clone()
    }

    async fn is_running(&self) -> bool {
        self.calls.push("session::is_running()");
        !self.event.is_set()
    }

    async fn stop(&self) {
        self.calls.push("session::stop()");
        self.event.set();
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
    let ws_client = WsClient {
        from_ws: broadcast::channel(16).0,
        to_ws: mpsc::channel(16).0,
    };

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
