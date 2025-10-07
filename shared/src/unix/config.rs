use anyhow::Result;

use crate::{
    config::{ActorConfiguration, Configuration},
    log,
};

#[derive(Default, Debug, Clone)]
pub struct UnixConfig {
    actor: Option<ActorConfiguration>,
}

impl Configuration for UnixConfig {
    fn load_config(&mut self) -> Result<ActorConfiguration> {
        // If not exists
        if !std::path::Path::new(CONFIG_PATH).exists() {
            return Ok(ActorConfiguration::default());
        }

        // TODO: maybe if invalid data, back it up and return default?
        let config_str = std::fs::read_to_string(CONFIG_PATH)?;
        let config: ActorConfiguration = toml::from_str(&config_str)?;
        self.actor = Some(config.clone());
        log::info!("Configuration loaded from {}", CONFIG_PATH);
        Ok(config)
    }

    // Note: Does not creates the intermediate keys, they must exist
    // So the installer must create them or use a PATH that is sure to exist (e.g. SOFTWARE)
    // The final key (UDSActor) will be created if not existing
    fn save_config(&mut self, config: &ActorConfiguration) -> Result<()> {
        let toml_str = toml::to_string(config)?;
        // Ensure folder exists or create it
        std::fs::create_dir_all(std::path::Path::new(CONFIG_PATH).parent().unwrap())?;
        std::fs::write(CONFIG_PATH, toml_str)?;
        self.actor = Some(config.clone());

        log::info!("Configuration saved to {}", CONFIG_PATH);
        Ok(())
    }

    fn clear_config(&mut self) -> Result<()> {
        std::fs::remove_file(CONFIG_PATH).ok();
        self.actor = None;
        log::info!("Configuration file {} removed", CONFIG_PATH);
        Ok(())
    }

    fn config(&mut self, force_reload: bool) -> Result<ActorConfiguration> {
        if force_reload || self.actor.is_none() {
            self.load_config()
        } else {
            Ok(self.actor.clone().unwrap())
        }
    }
}

pub fn new_config_loader() -> Box<dyn Configuration> {
    Box::new(UnixConfig::default())
}

#[cfg(not(test))]
const CONFIG_PATH: &str = "/etc/udsactor/config.yaml";

#[cfg(test)]
const CONFIG_PATH: &str = "/tmp/udsactor/config.yaml";
