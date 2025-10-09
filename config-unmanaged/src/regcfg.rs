use fltk::prelude::*;

use shared::{broker::api::types, config, log};

use crate::config_unmanaged_fltk::ConfigGui;

pub fn broker_api_config(hostname: &str, verify_ssl: bool) -> config::ActorConfiguration {
    config::ActorConfiguration {
        broker_url: format!("https://{hostname}/uds/rest/"),
        verify_ssl,
        actor_type: Some(config::ActorType::Managed),
        master_token: None,
        own_token: None,
        restrict_net: None,
        pre_command: None,
        runonce_command: None,
        post_command: None,
        log_level: 0,
        config: None,
        data: None,
    }
}

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

        let log_level: types::LogLevel = actor_cfg.log_level.into();

        cfg_window
            .choice_log_level
            .set_value(u8::from(log_level) as i32);
        cfg_window.choice_log_level.redraw();
        if let Some(pre_cmd) = actor_cfg.pre_command {
            cfg_window.input_preconnect_cmd.set_value(&pre_cmd);
        }
        if let Some(runonce_cmd) = actor_cfg.runonce_command {
            cfg_window.input_runonce_cmd.set_value(&runonce_cmd);
        }
        if let Some(post_cmd) = actor_cfg.post_command {
            cfg_window.input_postconfig_cmd.set_value(&post_cmd);
        }
    } else {
        log::debug!("No existing config found, using defaults");
    }

    // cfg_window.choice_ssl_validation.set_value(0);
    // cfg_window.input_uds_server.set_value("172.27.0.1:8443");
    cfg_window.input_username.set_value("test");
    cfg_window.input_password.set_value("test");
}
