use shared::{broker::api::types, config, log};
use crate::AppWindow;

pub fn fill_window_fields(ui: &AppWindow) {
    // Fill the fields from existing config
    log::debug!("Filling window fields from existing config");
    let mut config_storage = config::new_config_storage();
    let res = config_storage.config(false);
    if let Ok(actor_cfg) = res {
        log::debug!("Existing config found: {:?}", actor_cfg);
        
        ui.set_verify_ssl(actor_cfg.verify_ssl);
        
        if !actor_cfg.broker_url.is_empty() {
            // Remove https:// and /uds/rest/ if present
            let url = actor_cfg
                .broker_url
                .trim_start_matches("https://")
                .trim_end_matches("/uds/rest/");
            ui.set_server_host(url.into());
        }
        
        ui.set_service_token(
            actor_cfg
                .master_token
                .as_ref()
                .map_or("", |s| s.as_str())
                .into()
        );

        ui.set_net_restriction(actor_cfg.restrict_net.clone().unwrap_or_default().into());

        let log_level: types::LogLevel = actor_cfg.log_level.into();
        ui.set_active_log_level(u8::from(log_level) as i32);

        // If we have a valid token, enable the test button
        ui.set_test_enabled(!actor_cfg.token().is_empty());
        
        if let Some(ciphers) = actor_cfg.config.ssl_ciphers {
            ui.set_ssl_ciphers(slint::SharedString::from(ciphers));
        }
    } else {
        log::debug!("No existing config found, using defaults");
    }
}
