use crate::app::{Fields, LOG_LEVELS};
use shared::{broker::api::types, config, log};

/// Fills the form from the existing configuration. Returns whether the Test button should be enabled.
pub fn fill_fields(fields: &mut Fields) -> bool {
    log::debug!("Filling window fields from existing config");
    let Ok(actor_cfg) = config::new_config_storage().config(false) else {
        log::debug!("No existing config found, using defaults");
        return false;
    };
    log::debug!("Existing config found: {:?}", actor_cfg);
    let has_token = !actor_cfg.token().is_empty();

    fields.verify_ssl = actor_cfg.verify_ssl;
    if !actor_cfg.broker_url.is_empty() {
        fields.server_host = actor_cfg
            .broker_url
            .trim_start_matches("https://")
            .trim_end_matches("/uds/rest/")
            .to_string();
    }
    fields.service_token = actor_cfg.master_token.clone().unwrap_or_default();
    fields.net_restriction = actor_cfg.restrict_net.clone().unwrap_or_default();

    let log_level: types::LogLevel = actor_cfg.log_level.into();
    fields.log_level = (u8::from(log_level) as usize).min(LOG_LEVELS.len() - 1);

    if let Some(ciphers) = actor_cfg.config.ssl_ciphers {
        fields.ssl_ciphers = ciphers;
    }

    has_token
}
