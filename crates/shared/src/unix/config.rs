use anyhow::Result;

use crate::{
    config::{ActorConfiguration, Configuration},
    log,
};

const CONFIG_PATH: &str = "/etc/udsactor/";

#[derive(Default, Debug, Clone)]
pub struct UnixConfig {
    actor: Option<ActorConfiguration>,
}

fn get_config_file() -> String {
    if std::env::var("UDS_ACTOR_TEST").is_ok() {
        "/tmp/udsactor_test_config.cfg".to_string()
    } else {
        format!("{}/udsactor.cfg", CONFIG_PATH)
    }
}

impl Configuration for UnixConfig {
    fn load_config(&mut self) -> Result<ActorConfiguration> {
        // If not exists folder (in CONFIG_PATH), create it
        std::fs::create_dir_all(std::path::Path::new(CONFIG_PATH).parent().unwrap())?;
        let config_file = get_config_file();
        if !std::path::Path::new(&config_file).exists() {
            return Ok(ActorConfiguration::default());
        }

        // TODO: maybe if invalid data, back it up and return default?
        let config_str = std::fs::read_to_string(&config_file)?;
        let config: ActorConfiguration = toml::from_str(&config_str)?;
        self.actor = Some(config.clone());
        log::info!("Configuration loaded from {}", config_file);
        Ok(config)
    }

    // Note: Does not creates the intermediate keys, they must exist
    // So the installer must create them or use a PATH that is sure to exist (e.g. SOFTWARE)
    // The final key (UDSActor) will be created if not existing
    fn save_config(&mut self, config: &ActorConfiguration) -> Result<()> {
        self.actor = Some(config.clone());

        let config_file = get_config_file();

        let toml_str = toml::to_string(config)?;
        // Ensure folder exists or create it
        std::fs::create_dir_all(std::path::Path::new(&config_file).parent().unwrap())?;
        std::fs::write(&config_file, toml_str)?;

        log::info!("Configuration saved to {}", config_file);
        Ok(())
    }

    fn clear_config(&mut self) -> Result<()> {
        let config_file = get_config_file();
        std::fs::remove_file(&config_file).ok();
        self.actor = None;
        log::info!("Configuration file {} removed", config_file);
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

pub fn new_config_storage() -> Box<dyn Configuration> {
    Box::new(UnixConfig::default())
}
