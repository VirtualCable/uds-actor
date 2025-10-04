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
