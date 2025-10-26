use std::sync::{Arc, atomic::AtomicBool};
use tokio::sync::RwLock;

use shared::sync::OnceSignal;

#[derive(Clone)]
pub struct UserInfo {
    pub username: String,
    pub session_type: String,
    pub session_id: Option<String>,
}

#[derive(Clone)]
pub struct Platform {
    config: Arc<RwLock<shared::config::ActorConfiguration>>,
    operations: Arc<dyn shared::operations::Operations>, // Different for Windows, Linux, Mac, ...
    broker_api: Arc<RwLock<dyn shared::broker::api::BrokerApi>>,

    stop: OnceSignal,
    user_info: Arc<RwLock<Option<UserInfo>>>,
    restart_flag: Arc<AtomicBool>,
}

impl Platform {
    pub fn new(stop: OnceSignal, restart_flag: Arc<AtomicBool>) -> Self {
        let mut cfg = shared::config::new_config_storage();
        let cfg = cfg.config(true).unwrap(); // Forced load

        // If no config, panic, we need config
        let config = Arc::new(tokio::sync::RwLock::new(cfg.clone()));

        let operations = shared::operations::new_operations();
        // Release compilation will fail, because testing is not allowed in release builds, so if we forget this
        let broker_api = shared::broker::api::UdsBrokerApi::new(cfg, false, None);

        Self {
            config,
            operations,
            broker_api: Arc::new(tokio::sync::RwLock::new(broker_api)),
            stop,
            user_info: Arc::new(RwLock::new(None)),
            restart_flag,
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

    pub fn config_storage(&self) -> Box<dyn shared::config::Configuration> {
        shared::config::new_config_storage()
    }

    pub fn get_stop(&self) -> OnceSignal {
        self.stop.clone()
    }

    pub fn get_user_info(&self) -> Arc<RwLock<Option<UserInfo>>> {
        self.user_info.clone()
    }

    pub fn get_restart_flag(&self) -> Arc<AtomicBool> {
        self.restart_flag.clone()
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
            stop: OnceSignal::new(),
            user_info: Arc::new(RwLock::new(None)),
            restart_flag: Arc::new(AtomicBool::new(false)),
        }
    }
}
