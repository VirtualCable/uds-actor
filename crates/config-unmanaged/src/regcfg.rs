use fltk::prelude::*;

use shared::{broker::api::types, config, log};

use crate::config_unmanaged_fltk::ConfigGui;

pub fn fill_window_fields(cfg_window: &mut ConfigGui) {
    // Fill the fields from existing config
    log::debug!("Filling window fields from existing config");
    let mut config_storage = config::new_config_storage();
    let config = config_storage.config(false);
    if let Ok(actor_cfg) = config {
        log::debug!("Existing config found: {:?}", actor_cfg);
        if actor_cfg.verify_ssl {
            cfg_window.choice_ssl_validation.set_value(1);
        } else {
            cfg_window.choice_ssl_validation.set_value(0);
        }
        cfg_window.choice_ssl_validation.redraw();
        if !actor_cfg.broker_url.is_empty() {
            // Remove https:// and /uds/rest/ if present
            let url = actor_cfg
                .broker_url
                .trim_start_matches("https://")
                .trim_end_matches("/uds/rest/");
            cfg_window.input_uds_server.set_value(url);
        }
        cfg_window.input_token.set_value(
            actor_cfg
                .master_token
                .as_ref()
                .map_or("", |s| s.as_str()),
        );

        cfg_window
            .input_net
            .set_value(actor_cfg.restrict_net.clone().unwrap_or_default().as_str());

        let log_level: types::LogLevel = actor_cfg.log_level.into();

        cfg_window
            .choice_log_level
            .set_value(u8::from(log_level) as i32);
        cfg_window.choice_log_level.redraw();

        // If we have a valid token, enable the test button
        if actor_cfg.token().is_empty() {
            cfg_window.button_test.deactivate();
        } else {
            cfg_window.button_test.activate();
        }
    } else {
        log::debug!("No existing config found, using defaults");
    }
}
