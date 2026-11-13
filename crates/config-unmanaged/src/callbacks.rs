use std::time::Duration;

use shared::{broker::api::block, broker::api::types, config, log};

const BROKER_TIMEOUT: Duration = Duration::from_millis(1500);

pub fn ciphers_look_valid(ciphers: &str) -> bool {
    ciphers.is_empty() || ciphers.split(':').all(|part| !part.trim().is_empty())
}

fn optional(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

pub fn save_unmanaged(
    host: &str,
    verify_ssl: bool,
    token: &str,
    net: &str,
    log_level: types::LogLevel,
    ciphers: &str,
) -> Result<(), String> {
    let final_cfg = config::ActorConfiguration {
        broker_url: format!("https://{}/uds/rest/", host),
        verify_ssl,
        actor_type: config::ActorType::Unmanaged,
        master_token: optional(token),
        own_token: None,
        restrict_net: optional(net),
        pre_command: None,
        runonce_command: None,
        post_command: None,
        log_level: log_level.into(),
        config: config::ActorDataConfiguration {
            ssl_ciphers: optional(ciphers),
            ..Default::default()
        },
        data: None,
    };

    config::new_config_storage()
        .save_config(&final_cfg)
        .map(|()| log::debug!("Config saved successfully"))
        .map_err(|error| {
            log::error!("Failed to save config: {}", error);
            error.to_string()
        })
}

pub fn registered_config() -> Result<config::ActorConfiguration, String> {
    log::debug!("Test connection button clicked");
    let cfg = config::new_config_storage()
        .load_config()
        .map_err(|error| {
            log::error!("Failed to load existing config: {}", error);
            format!("Failed to load existing config: {error}")
        })?;
    if cfg.broker_url.is_empty() || cfg.token().is_empty() {
        return Err("Nothing to test: Only actors with tokens can be tested".to_string());
    }
    Ok(cfg)
}

pub fn test_connection(cfg: config::ActorConfiguration) -> Result<String, String> {
    block::test(cfg, Some(BROKER_TIMEOUT))
        .map(|msg| format!("Connection successful: {msg}"))
        .map_err(|error| format!("Connection failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::ciphers_look_valid;

    #[test]
    fn empty_or_well_formed_cipher_lists_are_valid() {
        assert!(ciphers_look_valid(""));
        assert!(ciphers_look_valid(
            "ECDHE-RSA-AES128-GCM-SHA256:ECDHE-RSA-AES256-GCM-SHA384"
        ));
    }

    #[test]
    fn empty_cipher_entries_are_invalid() {
        assert!(!ciphers_look_valid(
            "ECDHE-RSA-AES128-GCM-SHA256::ECDHE-RSA-AES256-GCM-SHA384"
        ));
        assert!(!ciphers_look_valid(":ECDHE-RSA-AES128-GCM-SHA256"));
    }
}
