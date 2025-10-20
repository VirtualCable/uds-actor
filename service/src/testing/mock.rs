use crate::platform::Platform;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

use shared::{
    config::{ActorConfiguration, ActorDataConfiguration, ActorType},
    testing::mock::{Calls, BrokerApiMock, OperationsMock},
    ws::{
        request_tracker::RequestTracker,
        server::ServerContext,
        types::{RpcEnvelope, RpcMessage},
    },
};

pub async fn mock_platform() -> (Platform, Calls) {
    let config = ActorConfiguration {
        broker_url: "https://localhost".to_string(),
        verify_ssl: true,
        actor_type: ActorType::Managed,
        master_token: None,
        own_token: None,
        restrict_net: None,
        pre_command: None,
        runonce_command: None,
        post_command: None,
        log_level: 0,
        config: ActorDataConfiguration::default(),
        data: None,
    };
    let calls = Calls::new();
    let operations = Arc::new(OperationsMock::new(calls.clone()));
    let broker_api = Arc::new(tokio::sync::RwLock::new(BrokerApiMock::new(calls.clone())));

    let platform = crate::platform::Platform::new_with_params(
        Some(config),
        Some(operations),
        Some(broker_api),
    );
    (platform, calls)
}

pub async fn mock_server_info() -> ServerContext {
    let (workers_tx, _workers_rx) = mpsc::channel::<RpcEnvelope<RpcMessage>>(128);
    let (wsclient_to_workers, _) = broadcast::channel::<RpcEnvelope<RpcMessage>>(128);
    let tracker = RequestTracker::new();

    ServerContext {
        to_ws: workers_tx,
        from_ws: wsclient_to_workers.clone(),
        tracker,
    }
}

pub async fn mock_server_info_with_worker_rx() -> (ServerContext, broadcast::Receiver<RpcEnvelope<RpcMessage>>) {
    let (workers_tx, _workers_rx) = mpsc::channel::<RpcEnvelope<RpcMessage>>(128);
    let (wsclient_to_workers, wsclient_to_workers_rx) =
        broadcast::channel::<RpcEnvelope<RpcMessage>>(128);
    let tracker = RequestTracker::new();

    (
        ServerContext {
            to_ws: workers_tx,
            from_ws: wsclient_to_workers.clone(),
            tracker,
        },
        wsclient_to_workers_rx,
    )
}
