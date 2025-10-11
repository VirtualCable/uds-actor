use std::sync::Arc;

#[derive(Clone)]
pub struct Platform {
    config: Arc<tokio::sync::RwLock<shared::config::ActorConfiguration>>,
    operations: Arc<dyn shared::operations::Operations>, // Different for Windows, Linux, Mac, ...
    broker_api: Arc<tokio::sync::RwLock<dyn shared::broker::api::BrokerApi>>,
}

impl Platform {
    pub fn new() -> Self {
        let mut cfg = shared::config::new_config_storage();
        let cfg = cfg.config(true).unwrap();

        // If no config, panic, we need config
        let config = Arc::new(tokio::sync::RwLock::new(cfg.clone()));

        // TODO: Restore real operations after development
        // let operations = shared::operations::new_operations();
        let operations = Arc::new(shared::testing::fake::FakeOperations::default());

        let broker_api = shared::broker::api::UdsBrokerApi::new(cfg, false, None);

        Self {
            config,
            operations,
            broker_api: Arc::new(tokio::sync::RwLock::new(broker_api)),
        }
    }

    pub fn operations(&self) -> Arc<dyn shared::operations::Operations> {
        self.operations.clone()
    }

    pub fn broker_api(&self) -> Arc<tokio::sync::RwLock<dyn shared::broker::api::BrokerApi>> {
        self.broker_api.clone()
    }

    pub fn config(&self) -> Arc<tokio::sync::RwLock<shared::config::ActorConfiguration>> {
        self.config.clone()
    }

    // Only for tests
    #[allow(dead_code)]
    #[cfg(test)]
    pub fn new_with_params(
        config: Option<shared::config::ActorConfiguration>,
        operations: Option<Arc<dyn shared::operations::Operations>>,
        broker_api: Option<Arc<tokio::sync::RwLock<dyn shared::broker::api::BrokerApi>>>,
    ) -> Self {
        let cfg = if let Some(cfg) = config {
            cfg
        } else {
            let mut cfg = shared::config::new_config_storage();
            cfg.config(true).unwrap()
        };
        let config = Arc::new(tokio::sync::RwLock::new(cfg.clone()));
        let operations = operations.unwrap_or_else(|| shared::operations::new_operations());
        let broker_api = broker_api.unwrap_or_else(|| {
            Arc::new(tokio::sync::RwLock::new(
                shared::broker::api::UdsBrokerApi::new(cfg, false, None),
            ))
        });

        Self {
            operations,
            broker_api,
            config,
        }
    }
}

impl Default for Platform {
    fn default() -> Self {
        Self::new()
    }
}
