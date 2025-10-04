use anyhow::Result;
use serde_yaml;

use crate::{
    config::{ActorConfiguration, ConfigLoader},
    log,
};

#[derive(Debug, Clone)]
pub struct UnixConfig {
    actor: Option<ActorConfiguration>,
}

impl Default for UnixConfig {
    fn default() -> Self {
        Self { actor: None }
    }
}

const CONFIG_PATH: &str = "/etc/udsactor/config.yaml";

impl ConfigLoader for UnixConfig {
    fn load_config(&mut self) -> Result<ActorConfiguration> {
        Err(anyhow::anyhow!("UnixConfig load_config not implemented"))
    }

    // Note: Does not creates the intermediate keys, they must exist
    // So the installer must create them or use a PATH that is sure to exist (e.g. SOFTWARE)
    // The final key (UDSActor) will be created if not existing
    fn save_config(&mut self, config: &ActorConfiguration) -> Result<()> {
        // Serialize ActorConfiguration to YAML
        let yaml = serde_yaml::to_string(config)
            .map_err(|e| anyhow::anyhow!("YAML serialization failed: {e}"))?;

        // Open file for writing (truncate if exists)
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(CONFIG_PATH)
            .map_err(|e| anyhow::anyhow!("Failed to open config file {CONFIG_PATH}: {e}"))?;

        // Write YAML content
        file.write_all(yaml.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to write config file: {e}"))?;

        // Update in-memory cache
        self.actor = Some(config.clone());

        Ok(())
    }

    fn clear_config(&mut self) -> Result<()> {
        Err(anyhow::anyhow!("UnixConfig clear_config not implemented"))
    }

    fn config(&mut self, force_reload: bool) -> Result<ActorConfiguration> {
        if force_reload || self.actor.is_none() {
            self.load_config()
        } else {
            Ok(self.actor.clone().unwrap())
        }
    }
}

pub fn new_config_loader() -> Box<dyn ConfigLoader> {
    Box::new(WindowsConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let root = RegistryRoot::CurrentUser; // Use CurrentUser for tests
        let mut config = WindowsConfig {
            actor: None,
            keys_root: root,
        };
        let test_cfg = get_test_config();
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
