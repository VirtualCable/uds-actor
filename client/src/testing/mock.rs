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
    fn new(calls: Calls, stop_signal: OnceSignal) -> Self {
        Self {
            event: stop_signal,
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

struct WsReqsMock {
    calls: Calls,
}

impl WsReqsMock {
    fn new(calls: Calls) -> Self {
        Self { calls }
    }
}

#[async_trait::async_trait]
impl crate::ws_reqs::WsReqs for WsReqsMock {
    async fn send_login(&self) -> anyhow::Result<shared::ws::types::LoginResponse> {
        self.calls.push("ws_reqs::send_login()");
        Ok(shared::ws::types::LoginResponse {
            ip: "127.0.0.1".to_string(),
            hostname: "mock_host".to_string(),
            deadline: None,
            max_idle: Some(600),
            session_id: Some("mock_session_id".to_string()),
        })
    }
    async fn send_logout(&self, _session_id: Option<&str>) -> anyhow::Result<()> {
        self.calls.push("ws_reqs::send_logout()");
        Ok(())
    }
}

pub async fn mock_platform(
    manager: Option<std::sync::Arc<dyn crate::session::SessionManagement>>,
    operations: Option<std::sync::Arc<dyn shared::operations::Operations>>,
    ws_requester: Option<std::sync::Arc<dyn crate::ws_reqs::WsReqs>>,
    stop_signal: Option<OnceSignal>,
    port: u16,
) -> (
    crate::platform::Platform,
    Calls,
    broadcast::Receiver<shared::ws::types::RpcEnvelope<shared::ws::types::RpcMessage>>,
    mpsc::Receiver<shared::ws::types::RpcEnvelope<shared::ws::types::RpcMessage>>,
) {
    let calls: Calls = Calls::new();
    let stop_signal = stop_signal.unwrap_or_default();
    let manager = manager.unwrap_or_else(|| {
        std::sync::Arc::new(SessionManagerMock::new(calls.clone(), stop_signal.clone()))
    });
    let operations =
        operations.unwrap_or_else(|| std::sync::Arc::new(OperationsMock::new(calls.clone())));

    let (from_ws, from_ws_receiver) =
        broadcast::channel::<shared::ws::types::RpcEnvelope<shared::ws::types::RpcMessage>>(32);
    let (to_ws, to_ws_receiver) =
        mpsc::channel::<shared::ws::types::RpcEnvelope<shared::ws::types::RpcMessage>>(32);
    let ws_client = WsClient { from_ws, to_ws };

    let ws_requester = ws_requester.unwrap_or_else(|| {
        std::sync::Arc::new(WsReqsMock::new(calls.clone()))
    });

    (
        crate::platform::Platform::new_with_params(
            Some(manager),
            Some(operations),
            Some(ws_client),
            Some(ws_requester),
            Some(stop_signal),
            port,
        )
        .await
        .unwrap(),
        calls,
        from_ws_receiver,
        to_ws_receiver,
    )
}
