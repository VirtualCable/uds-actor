use crate::platform::Platform;
use std::sync::Arc;

use shared::{
    config::ActorConfiguration,
    testing::fake::{Calls, FakeBrokerApi, FakeOperations},
};

pub async fn create_fake_platform() -> Platform {
    let config = ActorConfiguration {
        broker_url: "https://localhost".to_string(),
        verify_ssl: true,
        actor_type: Some(shared::config::ActorType::Managed),
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
    let operations = Arc::new(FakeOperations::new(Calls::new()));
    let broker_api = Arc::new(tokio::sync::RwLock::new(FakeBrokerApi::new(Calls::new())));

    crate::platform::Platform::new_with_params(Some(config), Some(operations), Some(broker_api))
}
