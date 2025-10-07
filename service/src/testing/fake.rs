use crate::platform::Platform;
use std::sync::Arc;

use shared::testing::fake::{Calls, FakeBrokerApi, FakeOperations};

pub async fn create_fake_platform() -> Platform {
    let config = {
        let mut cfg = shared::config::new_config_loader();
        cfg.config(true).unwrap()
    };
    let operations = Arc::new(FakeOperations::new(Calls::new()));
    let broker_api = Arc::new(tokio::sync::RwLock::new(FakeBrokerApi::new(Calls::new())));

    crate::platform::Platform::new_with_params(Some(config), Some(operations), Some(broker_api))
}
