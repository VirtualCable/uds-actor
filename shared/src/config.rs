use anyhow::Result;
use serde::{Deserialize, Serialize};

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
    pub host: String,
    pub check_certificate: bool,
    pub actor_type: Option<String>,
    pub master_token: Option<String>, // Configured master token. Will be replaced by unique one if unmanaged
    pub own_token: Option<String>, // On unmanaged, master_token will be cleared and this will be used (unique provided by server)
    pub restrict_net: Option<String>,
    pub pre_command: Option<String>,
    pub runonce_command: Option<String>,
    pub post_command: Option<String>,
    pub log_level: i32,
    pub config: Option<ActorDataConfiguration>,
    pub data: Option<serde_json::Value>,
}

pub trait ConfigLoader {
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

    #[cfg(target_family = "unix")]
    use crate::unix::config::new_test_config_loader;

    fn get_test_config() -> ActorConfiguration {
        ActorConfiguration {
            host: "https://example.com".to_string(),
            check_certificate: true,
            actor_type: Some("unmanaged".to_string()),
            master_token: Some("master123".to_string()),
            own_token: None,
            restrict_net: Some("192.168.1.0/24".to_string()),
            pre_command: None,
            runonce_command: None,
            post_command: None,
            log_level: 3,
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

        a.host == b.host
            && a.check_certificate == b.check_certificate
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
        let mut config = new_test_config_loader();
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
