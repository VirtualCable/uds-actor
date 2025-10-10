use fltk::prelude::*;

use shared::{broker::api::types, config, log};

use crate::config_unmanaged_fltk::ConfigGui;

/// Callback for the "Register" button
/// - Validate fields
/// - Login to API
/// - Register the actor
pub fn bnt_save_clicked(cfg_window: &ConfigGui) {
    let uds_server = cfg_window.input_uds_server.value().trim().to_string();

    let token = cfg_window.input_token.value().trim().to_string();
    let net = cfg_window.input_net.value().trim().to_string(); // Can be enpty
    let log_level: types::LogLevel = (cfg_window.choice_log_level.value() as u8).min(4).into();

    if uds_server.is_empty() || token.is_empty() {
        fltk::dialog::alert_default("Hostname and token are required");
        return;
    }

    let final_cfg = config::ActorConfiguration {
        broker_url: format!("https://{}/uds/rest/", uds_server),
        verify_ssl: cfg_window.choice_ssl_validation.value() == 1,
        actor_type: Some(config::ActorType::Unmanaged),
        master_token: Some(token),
        own_token: None,
        restrict_net: Some(net),
        pre_command: None,
        runonce_command: None,
        post_command: None,
        log_level: log_level.into(),
        config: None,
        data: None,
    };

    let mut config_storage = config::new_config_storage();
    if let Err(e) = config_storage.save_config(&final_cfg) {
        fltk::dialog::alert_default(&format!("Failed to save config: {}", e));
        log::error!("Failed to save config: {}", e);
    } else {
        log::debug!("Config saved successfully");
    }
}
