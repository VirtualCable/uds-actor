use shared::{broker::api::types, config, log};
use crate::AppWindow;

pub fn broker_api_config(hostname: &str, verify_ssl: bool, ciphers: &str) -> config::ActorConfiguration {
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
            ssl_ciphers: if ciphers.is_empty() { None } else { Some(ciphers.to_string()) },
            ..Default::default()
        },
        data: None,
    }
}

pub fn fill_window_fields(ui: &AppWindow) {
    // Fill the fields from existing config
    log::debug!("Filling window fields from existing config");
    let mut config_storage = config::new_config_storage();
    let res = config_storage.config(false);
    if let Ok(actor_cfg) = res {
        log::debug!("Existing config found: {:?}", actor_cfg);
        
        // If we have a valid token, enable the test button
        ui.set_test_enabled(!actor_cfg.token().is_empty());

        ui.set_verify_ssl(actor_cfg.verify_ssl);

        if !actor_cfg.broker_url.is_empty() {
            // Remove https:// and /uds/rest/ if present
            let url = actor_cfg
                .broker_url
                .trim_start_matches("https://")
                .trim_end_matches("/uds/rest/");
            ui.set_server_host(url.into());
        }

        let log_level: types::LogLevel = actor_cfg.log_level.into();
        ui.set_active_log_level(u8::from(log_level) as i32);

        if let Some(pre_cmd) = actor_cfg.pre_command {
            ui.set_preconnect_cmd(pre_cmd.into());
        }
        if let Some(runonce_cmd) = actor_cfg.runonce_command {
            ui.set_runonce_cmd(runonce_cmd.into());
        }
        if let Some(post_cmd) = actor_cfg.post_command {
            ui.set_postconfig_cmd(post_cmd.into());
        }
        
        if let Some(ciphers) = actor_cfg.config.ssl_ciphers {
            ui.set_ssl_ciphers(slint::SharedString::from(ciphers));
        }
    } else {
        log::debug!("No existing config found, using defaults");
    }
}
