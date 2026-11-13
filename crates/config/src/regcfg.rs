use crate::app::{Fields, LOG_LEVELS};
use shared::{broker::api::types, config, log};

pub fn broker_api_config(
    hostname: &str,
    verify_ssl: bool,
    ciphers: &str,
) -> config::ActorConfiguration {
    config::ActorConfiguration {
        broker_url: format!("https://{hostname}/uds/rest/"),
        verify_ssl,
        actor_type: config::ActorType::Managed,
        master_token: None,
        own_token: None,
        restrict_net: None,
        pre_command: None,
        runonce_command: None,
        post_command: None,
        log_level: 0,
        config: config::ActorDataConfiguration {
            ssl_ciphers: if ciphers.is_empty() {
                None
            } else {
                Some(ciphers.to_string())
            },
            ..Default::default()
        },
        data: None,
    }
}

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

    let log_level: types::LogLevel = actor_cfg.log_level.into();
    fields.log_level = (u8::from(log_level) as usize).min(LOG_LEVELS.len() - 1);

    if let Some(pre_cmd) = actor_cfg.pre_command {
        fields.preconnect_cmd = pre_cmd;
    }
    if let Some(runonce_cmd) = actor_cfg.runonce_command {
        fields.runonce_cmd = runonce_cmd;
    }
    if let Some(post_cmd) = actor_cfg.post_command {
        fields.postconfig_cmd = post_cmd;
    }
    if let Some(ciphers) = actor_cfg.config.ssl_ciphers {
        fields.ssl_ciphers = ciphers;
    }

    has_token
}
