use std::time::Duration;

use shared::{
    broker::api::{block, types},
    config, log,
    system::{NetworkInterface, System},
};

use crate::regcfg;

const BROKER_TIMEOUT: Duration = Duration::from_millis(1500);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Register,
    Test,
}

pub struct RegistrationInput {
    pub hostname: String,
    pub verify_ssl: bool,
    pub auth: String,
    pub username: String,
    pub password: String,
    pub ciphers: String,
    pub log_level: types::LogLevel,
    pub pre_command: String,
    pub runonce_command: String,
    pub post_command: String,
    pub interface: NetworkInterface,
}

pub fn ciphers_look_valid(ciphers: &str) -> bool {
    ciphers.is_empty() || ciphers.split(':').all(|part| !part.trim().is_empty())
}

fn optional(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

pub fn enumerate_authenticators(
    hostname: &str,
    verify_ssl: bool,
) -> Result<Vec<types::Authenticator>, String> {
    let actor_cfg = regcfg::broker_api_config(hostname, verify_ssl, "");
    match block::enumerate_authenticators(actor_cfg, Some(BROKER_TIMEOUT)) {
        Ok(mut auths) => {
            auths.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(auths)
        }
        Err(error) => {
            log::warn!("Authenticator enumeration failed: {}", error);
            Err(error.to_string())
        }
    }
}

pub fn register(input: RegistrationInput, operations: &dyn System) -> Result<String, String> {
    let actor_cfg = regcfg::broker_api_config(&input.hostname, input.verify_ssl, &input.ciphers);

    let token = block::api_login(
        actor_cfg.clone(),
        &input.auth,
        &input.username,
        &input.password,
    )
    .map_err(|error| {
        log::error!("Login failed: {}", error);
        format!("Login failed: {error}")
    })?;
    log::debug!("Login successful, got token");

    let username_full = format!("{}@{}", input.username, input.auth);
    let os = operations.get_os_version().unwrap_or_default();
    let computer_name = operations.get_computer_name().unwrap_or_default();

    let reg_auth = types::RegisterRequest {
        version: shared::consts::VERSION,
        build: shared::consts::BUILD,
        hostname: &computer_name,
        username: &username_full,
        ip: &input.interface.ip_addr,
        mac: &input.interface.mac,
        commands: types::RegisterCommands {
            pre_command: optional(&input.pre_command),
            runonce_command: optional(&input.runonce_command),
            post_command: optional(&input.post_command),
        },
        log_level: input.log_level.into(),
        os: &os,
    };

    let master_token = block::register(actor_cfg.clone(), &reg_auth, &token).map_err(|error| {
        log::error!("Registration failed: {}", error);
        format!("Registration failed: {error}")
    })?;
    log::debug!("Registration successful");

    let final_cfg = config::ActorConfiguration {
        broker_url: format!("https://{}/uds/rest/", input.hostname),
        verify_ssl: actor_cfg.verify_ssl,
        actor_type: config::ActorType::Managed,
        master_token: Some(master_token),
        own_token: None,
        restrict_net: None,
        pre_command: reg_auth.commands.pre_command.clone(),
        runonce_command: reg_auth.commands.runonce_command.clone(),
        post_command: reg_auth.commands.post_command.clone(),
        log_level: input.log_level.into(),
        config: config::ActorDataConfiguration {
            ssl_ciphers: optional(&input.ciphers),
            ..Default::default()
        },
        data: None,
    };

    config::new_config_storage()
        .save_config(&final_cfg)
        .map_err(|error| format!("Failed to save config: {error}"))?;

    Ok("Registration successful".to_string())
}

pub fn registered_config() -> Result<config::ActorConfiguration, String> {
    log::debug!("Test connection button clicked");
    let cfg = config::new_config_storage()
        .load_config()
        .map_err(|error| format!("Failed to load existing config: {error}"))?;
    if cfg.broker_url.is_empty() || cfg.token().is_empty() {
        return Err("Please register with UDS before testing the connection".to_string());
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
