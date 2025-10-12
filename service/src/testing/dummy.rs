use crate::platform::Platform;
use std::sync::Arc;

use shared::{
    config::{ActorConfiguration, ActorType},
    testing::dummy::{Calls, DummyBrokerApi, DummyOperations},
};

pub async fn create_dummy_platform() -> (Platform, Calls) {
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
        config: None,
        data: None,
    };
    let calls = Calls::new();
    let operations = Arc::new(DummyOperations::new(calls.clone()));
    let broker_api = Arc::new(tokio::sync::RwLock::new(DummyBrokerApi::new(calls.clone())));

    let platform = crate::platform::Platform::new_with_params(Some(config), Some(operations), Some(broker_api));
    (platform, calls)
}
