use std::sync::Arc;

#[derive(Clone)]
pub struct Platform {
    operations: Arc<dyn shared::operations::Operations>, // Different for Windows, Linux, Mac, ...
    broker_api: Arc<tokio::sync::RwLock<shared::broker::api::BrokerApi>>,
}

impl Platform {
    pub fn new() -> Self {
        let operations = shared::operations::new_operations();
        // TODO: Config for BrokerApi from config data
        let broker_api = Arc::new(tokio::sync::RwLock::new(
            shared::broker::api::BrokerApi::new("", false, std::time::Duration::from_secs(5), true),
        ));

        Self {
            operations,
            broker_api,
        }
    }

    pub fn operations(&self) -> Arc<dyn shared::operations::Operations> {
        self.operations.clone()
    }

    pub fn broker_api(&self) -> Arc<tokio::sync::RwLock<shared::broker::api::BrokerApi>> {
        self.broker_api.clone()
    }
    // Only for tests
    #[cfg(test)]
    pub fn new_with_params(
        operations: Option<Arc<dyn shared::operations::Operations>>,
        broker_api: Option<Arc<tokio::sync::RwLock<shared::broker::api::BrokerApi>>>,
    ) -> Self {
        let operations = operations.unwrap_or_else(|| shared::operations::new_operations());
        let broker_api = broker_api.unwrap_or_else(|| {
            Arc::new(tokio::sync::RwLock::new(
                shared::broker::api::BrokerApi::new(
                    "",
                    false,
                    std::time::Duration::from_secs(5),
                    true,
                ),
            ))
        });

        Self {
            operations,
            broker_api,
        }
    }
}

impl Default for Platform {
    fn default() -> Self {
        Self::new()
    }
}
