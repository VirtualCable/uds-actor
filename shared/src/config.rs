use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Actor types
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActorType {
    #[default]
    Managed,
    Unmanaged,
}

impl From<&str> for ActorType {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "managed" => ActorType::Managed,
            "unmanaged" => ActorType::Unmanaged,
            _ => ActorType::Unmanaged,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ActorOsConfiguration {
    pub action: String,
    pub name: String,
    pub custom: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ActorDataConfiguration {
    pub unique_id: Option<String>,
    pub os: Option<ActorOsConfiguration>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ActorConfiguration {
    pub broker_url: String,
    pub verify_ssl: bool,
    pub actor_type: Option<ActorType>,
    pub master_token: Option<String>, // Configured master token. Will be replaced by unique one if unmanaged
    pub own_token: Option<String>, // On unmanaged, master_token will be cleared and this will be used (unique provided by server)
    pub restrict_net: Option<String>,
    pub pre_command: Option<String>,
    pub runonce_command: Option<String>,
    pub post_command: Option<String>,
    pub log_level: i32,
    pub timeout: Option<u64>, // Timeout for API calls, in seconds
    pub no_proxy: bool,       // If true, do not use proxy from env vars
    // Additional configuration data from server
    pub config: Option<ActorDataConfiguration>,
    pub data: Option<serde_json::Value>,
}

impl ActorConfiguration {
    pub fn token(&self) -> String {
        if let Some(token) = self.master_token.clone() {
            token
        } else {
            self.own_token.as_deref().unwrap_or("").to_string()
        }
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout.unwrap_or(10))
    }

    pub fn no_proxy(&self) -> bool {
        self.no_proxy
    }
}

pub trait Configuration {
    fn load_config(&mut self) -> Result<ActorConfiguration>;
    fn save_config(&mut self, config: &ActorConfiguration) -> Result<()>;
    fn clear_config(&mut self) -> Result<()>; // Remove saved config

    // Convenience method to get configuration
    // We could cache config on most cases, to not reload it every time
    fn config(&mut self, _force_reload: bool) -> Result<ActorConfiguration> {
        self.load_config()
    }
}

#[cfg(target_os = "windows")]
pub use crate::windows::config::new_config_loader;

#[cfg(target_family = "unix")]
pub use crate::unix::config::new_config_loader;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log;

    fn get_test_config() -> ActorConfiguration {
        ActorConfiguration {
            broker_url: "https://example.com".to_string(),
            verify_ssl: true,
            actor_type: Some(ActorType::Managed),
            master_token: Some("master123".to_string()),
            own_token: None,
            restrict_net: Some("192.168.1.0/24".to_string()),
            pre_command: None,
            runonce_command: None,
            post_command: None,
            log_level: 3,
            timeout: Some(15),
            no_proxy: false,
            config: None,
            data: None,
        }
    }

    fn compare_configs(a: &ActorConfiguration, b: &ActorConfiguration) -> bool {
        // Compare a.config to b.config (Option<ActorDataConfiguration>)
        if a.config.is_none() != b.config.is_none() {
            return false;
        }
        if let (Some(a_cfg), Some(b_cfg)) = (&a.config, &b.config) {
            if a_cfg.unique_id != b_cfg.unique_id {
                return false;
            }
            if a_cfg.os.is_none() != b_cfg.os.is_none() {
                return false;
            }
            if let (Some(a_os), Some(b_os)) = (&a_cfg.os, &b_cfg.os)
                && (a_os.action != b_os.action
                    || a_os.name != b_os.name
                    || a_os.custom != b_os.custom)
            {
                return false;
            }
        }

        a.broker_url == b.broker_url
            && a.verify_ssl == b.verify_ssl
            && a.actor_type == b.actor_type
            && a.master_token == b.master_token
            && a.own_token == b.own_token
            && a.restrict_net == b.restrict_net
            && a.pre_command == b.pre_command
            && a.runonce_command == b.runonce_command
            && a.post_command == b.post_command
            && a.log_level == b.log_level
            && a.timeout == b.timeout
            && a.no_proxy == b.no_proxy
    }

    #[test]
    fn test_registry_save_load_delete_config() {
        log::setup_logging("debug", crate::log::LogType::Tests);

        let test_cfg = get_test_config();
        let mut config = new_config_loader();
        let res = config.save_config(&test_cfg);
        assert!(res.is_ok(), "Failed to save config: {:?}", res.err());
        let loaded_cfg = config.load_config().unwrap();
        assert!(
            compare_configs(&test_cfg, &loaded_cfg),
            "Loaded config does not match saved config"
        );
        let res = config.clear_config();
        assert!(res.is_ok(), "Failed to clear config: {:?}", res.err());
        let cleared_cfg = config.load_config().unwrap();
        assert!(
            compare_configs(&cleared_cfg, &ActorConfiguration::default()),
            "Cleared config is not default"
        );
    }
}
