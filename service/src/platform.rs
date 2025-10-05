use std::sync::Arc;

#[derive(Clone)]
pub struct Platform {
    config: shared::config::ActorConfiguration,
    operations: Arc<dyn shared::operations::Operations>, // Different for Windows, Linux, Mac, ...
    broker_api: Arc<tokio::sync::RwLock<shared::broker::api::BrokerApi>>,
}

impl Platform {
    pub fn new() -> Self {
        let mut config = shared::config::new_config_loader();
        // If no config, panic, we need config
        let config = config.config(true).unwrap();

        let operations = shared::operations::new_operations();
        // TODO: Config for BrokerApi from config data
        let broker_api = Arc::new(tokio::sync::RwLock::new(
            shared::broker::api::BrokerApi::new(
                &config.host,
                config.verify_ssl,
                std::time::Duration::from_secs(5),
                true,
                config.actor_type.clone()
            ),
        ));

        Self {
            operations,
            broker_api,
            config,
        }
    }

    pub fn operations(&self) -> Arc<dyn shared::operations::Operations> {
        self.operations.clone()
    }

    pub fn broker_api(&self) -> Arc<tokio::sync::RwLock<shared::broker::api::BrokerApi>> {
        self.broker_api.clone()
    }

    pub fn config(&self) -> &shared::config::ActorConfiguration {
        &self.config
    }

    // Only for tests
    #[allow(dead_code)]
    #[cfg(test)]
    pub fn new_with_params(
        config: Option<shared::config::ActorConfiguration>,
        operations: Option<Arc<dyn shared::operations::Operations>>,
        broker_api: Option<Arc<tokio::sync::RwLock<shared::broker::api::BrokerApi>>>,
    ) -> Self {
        let config = config.unwrap_or_else(|| {
            let mut cfg = shared::config::new_config_loader();
            cfg.config(true).unwrap()
        });
        let operations = operations.unwrap_or_else(|| shared::operations::new_operations());
        let broker_api = broker_api.unwrap_or_else(|| {
            Arc::new(tokio::sync::RwLock::new(
                shared::broker::api::BrokerApi::new(
                    "",
                    false,
                    std::time::Duration::from_secs(5),
                    true,
                    config.actor_type.clone()
                ),
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
