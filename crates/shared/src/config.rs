use std::ops::Deref;

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
// NOTE: Debug is implemented manually (not derived) so secret-bearing values inside
// `custom` (e.g. password, new_password) can never reach any log sink.
#[derive(Clone, Default, Deserialize, Serialize)]
pub struct ActorOsConfiguration {
    #[serde(default)]
    pub action: ActorOsAction, // Default is None
    #[serde(default)]
    pub name: String, // Default is empty
    pub custom: Option<serde_json::Value>, // custom data depends on action
}

impl std::fmt::Debug for ActorOsConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let redacted_custom = self.custom.as_ref().map(|v| match v {
            serde_json::Value::Object(map) => serde_json::Value::Object(
                map.iter()
                    .map(|(k, val)| {
                        let masked = if k.to_lowercase().contains("password") {
                            serde_json::Value::String("[REDACTED]".to_string())
                        } else {
                            val.clone()
                        };
                        (k.clone(), masked)
                    })
                    .collect::<serde_json::Map<String, serde_json::Value>>(),
            ),
            other => other.clone(),
        });
        f.debug_struct("ActorOsConfiguration")
            .field("action", &self.action)
            .field("name", &self.name)
            .field("custom", &redacted_custom)
            .finish()
    }
}

impl Deref for ActorOsConfiguration {
    type Target = Self;

    fn deref(&self) -> &Self::Target {
        self
    }
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ActorDataConfiguration {
    pub unique_id: Option<String>,
    pub os: Option<ActorOsConfiguration>,
    pub ssl_ciphers: Option<String>,
}

// NOTE: Debug is implemented manually (not derived) so master_token/own_token/data
// can never reach any log sink through a {:?} render (mirrors JoinDomainOptions).
#[derive(Clone, Deserialize, Serialize)]
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

// Diagnostic-friendly masking for high-entropy values (broker tokens): keep the first
// and last 4 chars so the same token can be correlated across logs, but only when the
// value is longer than 8 chars — at 8 or fewer the preview would leak a large share of
// the entropy (defensive against malformed/malicious short submissions), so those are
// fully redacted. Uses char boundaries, never byte slices, to stay UTF-8 safe.
fn token_preview(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 8 {
        return "[REDACTED]".to_string();
    }
    let head: String = chars[..4].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}…{tail}")
}

impl std::fmt::Debug for ActorConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ActorConfiguration")
            .field("broker_url", &self.broker_url)
            .field("verify_ssl", &self.verify_ssl)
            .field("actor_type", &self.actor_type)
            .field(
                "master_token",
                &self.master_token.as_deref().map(token_preview),
            )
            .field(
                "own_token",
                &self.own_token.as_deref().map(token_preview),
            )
            .field("restrict_net", &self.restrict_net)
            .field("pre_command", &self.pre_command)
            .field("runonce_command", &self.runonce_command)
            .field("post_command", &self.post_command)
            .field("log_level", &self.log_level)
            .field("config", &self.config)
            .field("data", &self.data.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

impl Default for ActorConfiguration {
    fn default() -> Self {
        Self {
            broker_url: String::new(),
            // A missing or cleared configuration must never express "do not
            // validate"; disabling certificate verification is always an
            // explicit operator choice.
            verify_ssl: true,
            actor_type: ActorType::default(),
            master_token: None,
            own_token: None,
            restrict_net: None,
            pre_command: None,
            runonce_command: None,
            post_command: None,
            log_level: 50000, // ERROR
            config: ActorDataConfiguration::default(),
            data: None,
        }
    }
}

impl ActorConfiguration {
    pub fn token(&self) -> String {
        // Master token has precedence over master token
        if let Some(token) = self.master_token.clone()
            && !token.is_empty()
        {
            token
        } else {
            self.own_token.as_deref().unwrap_or("").to_string()
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.broker_url.is_empty() && !self.token().is_empty()
    }

    pub fn log_level(&self) -> crate::broker::api::types::LogLevel {
        self.log_level.into()
    }

    pub fn ssl_ciphers(&self) -> Option<&str> {
        self.config.ssl_ciphers.as_deref()
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

    #[test]
    fn test_debug_render_redacts_secrets() {
        let mut cfg = get_test_config();
        cfg.own_token = Some("own456".to_string());
        cfg.data = Some(serde_json::json!({"secret": "data-secret-value"}));
        cfg.config.os = Some(ActorOsConfiguration {
            action: ActorOsAction::Rename,
            name: "test-pc".to_string(),
            custom: Some(serde_json::json!({
                "username": "join-account",
                "password": "join-password-value",
                "new_password": "new-password-value"
            })),
        });

        let rendered = format!("{:?}", cfg);
        assert!(!rendered.contains("master123"), "master_token leaked through Debug");
        assert!(!rendered.contains("own456"), "own_token leaked through Debug");
        assert!(!rendered.contains("data-secret-value"), "data leaked through Debug");
        assert!(
            !rendered.contains("join-password-value"),
            "os.custom password leaked through Debug"
        );
        assert!(
            !rendered.contains("new-password-value"),
            "os.custom new_password leaked through Debug"
        );
        assert!(rendered.contains("test-pc"), "non-secret fields should render");
        assert!(rendered.contains("join-account"), "non-secret custom keys should render");
        assert!(rendered.contains("[REDACTED]"), "redaction markers should be present");
    }

    #[test]
    fn test_token_preview_boundaries() {
        // Short tokens (<= 8 chars) are fully redacted: a preview would leak too much
        // of the entropy (defensive against maliciously short submissions).
        assert_eq!(token_preview("12345678"), "[REDACTED]");
        // Just over the boundary: first 4 + last 4 visible, the middle never is.
        let nine = "abcdefghi";
        let p = token_preview(nine);
        assert!(p.starts_with("abcd") && p.ends_with("fghi") && !p.contains("e"));
        // Realistic long token: preview correlates head/tail, hides the body.
        let long = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(token_preview(long), "0123…4567");
        // UTF-8 safety: multibyte values are handled on char boundaries.
        let utf8 = "áéíóúñü€ŧøæåtoken-value-æøåŧ";
        let p = token_preview(utf8);
        assert!(p.chars().count() < utf8.chars().count() + 8);
        assert!(p.contains('…'));
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
        unsafe { std::env::set_var("UDS_ACTOR_TEST", "1") };

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
