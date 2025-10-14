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

impl ActorType {
    pub fn is_managed(&self) -> bool {
        *self == ActorType::Managed
    }
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

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum ActorOsAction {
    #[default]
    #[serde(rename = "none")]
    None,
    #[serde(rename = "rename")]
    Rename,
    #[serde(rename = "rename_ad")]
    JoinDomain,
}

// To keep compat with older versions, we accept empty json as our default
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ActorOsConfiguration {
    #[serde(default)]
    pub action: ActorOsAction,  // Default is None
    #[serde(default)]
    pub name: String,           // Default is empty
    pub custom: Option<serde_json::Value>,  // custom data depends on action
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
    pub actor_type: ActorType,
    pub master_token: Option<String>, // Configured master token. Will be replaced by unique one if unmanaged
    pub own_token: Option<String>, // On unmanaged, master_token will be cleared and this will be used (unique provided by server)
    pub restrict_net: Option<String>,
    pub pre_command: Option<String>,
    pub runonce_command: Option<String>,
    pub post_command: Option<String>,
    pub log_level: u32,
    // Additional configuration data from server
    pub config: ActorDataConfiguration,
    pub data: Option<serde_json::Value>,
}

impl ActorConfiguration {
    pub fn token(&self) -> String {
        // Own token has precedence over master token
        if let Some(token) = self.own_token.clone() {
            token
        } else {
            self.master_token.as_deref().unwrap_or("").to_string()
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.broker_url.is_empty() && !self.token().is_empty()
    }

    pub fn already_initialized(&self) -> bool {
        self.own_token.is_some()
    }
}

pub trait Configuration: Send + Sync + 'static {
    fn load_config(&mut self) -> Result<ActorConfiguration>;

    // Save config must ensure that, even if it cannot save the config, the in-memory
    // representation is updated
    fn save_config(&mut self, config: &ActorConfiguration) -> Result<()>;
    fn clear_config(&mut self) -> Result<()>; // Remove saved config

    // Obtain the current config, optionally forcing a reload from storage
    // If no config already loaded, it will load it
    fn config(&mut self, _force_reload: bool) -> Result<ActorConfiguration> {
        self.load_config()
    }
}

#[cfg(target_os = "windows")]
pub use crate::windows::config::new_config_storage;

#[cfg(target_family = "unix")]
pub use crate::unix::config::new_config_storage;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log;

    fn get_test_config() -> ActorConfiguration {
        ActorConfiguration {
            broker_url: "https://example.com".to_string(),
            verify_ssl: true,
            actor_type: ActorType::default(),
            master_token: Some("master123".to_string()),
            own_token: None,
            restrict_net: Some("192.168.1.0/24".to_string()),
            pre_command: None,
            runonce_command: None,
            post_command: None,
            log_level: 3,
            config: ActorDataConfiguration::default(),
            data: None,
        }
    }

    fn compare_configs(a: &ActorConfiguration, b: &ActorConfiguration) -> bool {
        // Compare a.config to b.config (Option<ActorDataConfiguration>)

        if a.config.unique_id != b.config.unique_id {
            return false;
        }
        if let (Some(a_os), Some(b_os)) = (&a.config.os, &b.config.os)
            && (a_os.action != b_os.action || a_os.name != b_os.name || a_os.custom != b_os.custom)
        {
            return false;
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
    }

    #[test]
    fn test_registry_save_load_delete_config() {
        log::setup_logging("debug", crate::log::LogType::Tests);

        let test_cfg = get_test_config();
        let mut config = new_config_storage();
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
