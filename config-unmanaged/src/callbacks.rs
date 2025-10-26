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

    if uds_server.is_empty() {
        fltk::dialog::alert_default("Hostname is required");
        return;
    }

    let final_cfg = config::ActorConfiguration {
        broker_url: format!("https://{}/uds/rest/", uds_server),
        verify_ssl: cfg_window.choice_ssl_validation.value() == 1,
        actor_type: config::ActorType::Unmanaged,
        master_token: Some(token),
        own_token: None,
        restrict_net: Some(net),
        pre_command: None,
        runonce_command: None,
        post_command: None,
        log_level: log_level.into(),
        config: config::ActorDataConfiguration::default(),
        data: None,
    };

    let mut config_storage = config::new_config_storage();
    if let Err(e) = config_storage.save_config(&final_cfg) {
        fltk::dialog::alert_default(&format!("Failed to save config: {}", e));
        log::error!("Failed to save config: {}", e);
    } else {
        fltk::dialog::message_default("Configuration saved successfully!\n");
        let mut btn_test = cfg_window.button_test.clone();
        btn_test.activate(); // Enable test button
        log::debug!("Config saved successfully");
    }
}

pub fn btn_test_clicked(cfg_window: &ConfigGui) {
    log::debug!("Test connection button clicked");
    let cfg = config::new_config_storage().load_config();
    if let Err(err) = cfg {
        fltk::dialog::alert_default(&format!("Failed to load existing config: {}", err));
        log::error!("Failed to load existing config: {}", err);
        return;
    }
    // Must have uds_server & token
    let actor_cfg = cfg.unwrap();
    if actor_cfg.broker_url.is_empty() || actor_cfg.token().is_empty() {
        fltk::dialog::alert_default("Nothing to test: Only actors with tokens can be tested");
        return;
    }

    match shared::broker::api::block::test(actor_cfg, Some(std::time::Duration::from_millis(800))) {
        Ok(msg) => {
            fltk::dialog::message_default(&format!("Connection successful:\n{}", msg));
            log::debug!("Connection test successful: {}", msg);
        }
        Err(e) => {
            fltk::dialog::alert_default(&format!("Connection failed:\n{}", e));
            log::error!("Connection test failed: {}", e);
            // Disable again if it fails
            let mut btn_test = cfg_window.button_test.clone();
            btn_test.deactivate();

        }
    }
}
