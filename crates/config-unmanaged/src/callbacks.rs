use crate::AppWindow;
use shared::{broker::api::types, config, log};
use slint::ComponentHandle;

/// Callback for the "Save" button
pub fn bnt_save_clicked(ui: &AppWindow) {
    let uds_server = ui.get_server_host().trim().to_string();
    let token = ui.get_service_token().trim().to_string();
    let net = ui.get_net_restriction().trim().to_string();
    let log_level_idx = ui.get_active_log_level();
    let log_level: types::LogLevel = (log_level_idx as u8).min(4).into();
    let ciphers = ui.get_ssl_ciphers().trim().to_string();

    if uds_server.is_empty() {
        let _ = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Validation Error")
            .set_description("Hostname is required")
            .show();
        return;
    }

    let final_cfg = config::ActorConfiguration {
        broker_url: format!("https://{}/uds/rest/", uds_server),
        verify_ssl: ui.get_verify_ssl(),
        actor_type: config::ActorType::Unmanaged,
        master_token: if token.is_empty() { None } else { Some(token) },
        own_token: None,
        restrict_net: if net.is_empty() { None } else { Some(net) },
        pre_command: None,
        runonce_command: None,
        post_command: None,
        log_level: log_level.into(),
        config: config::ActorDataConfiguration {
            ssl_ciphers: if ciphers.is_empty() {
                None
            } else {
                Some(ciphers)
            },
            ..Default::default()
        },
        data: None,
    };

    let mut config_storage = config::new_config_storage();
    if let Err(e) = config_storage.save_config(&final_cfg) {
        log::error!("Failed to save config: {}", e);
        let _ = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Save Error")
            .set_description(format!("Failed to save config: {}", e))
            .show();
    } else {
        log::debug!("Config saved successfully");
        ui.set_test_enabled(true);
        let _ = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Info)
            .set_title("Success")
            .set_description("Configuration saved successfully!")
            .show();
    }
}

pub fn btn_test_clicked(ui: &AppWindow) {
    log::debug!("Test connection button clicked");
    let cfg_res = config::new_config_storage().load_config();
    if let Err(err) = cfg_res {
        log::error!("Failed to load existing config: {}", err);
        let _ = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Config Error")
            .set_description(format!("Failed to load existing config: {}", err))
            .show();
        return;
    }

    let actor_cfg = cfg_res.unwrap();
    if actor_cfg.broker_url.is_empty() || actor_cfg.token().is_empty() {
        let _ = rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Warning)
            .set_title("No Token")
            .set_description("Nothing to test: Only actors with tokens can be tested")
            .show();
        return;
    }

    ui.set_loading(true);
    ui.set_status_text("Testing connection...".into());
    let ui_handle = ui.as_weak();

    std::thread::spawn(move || {
        match shared::broker::api::block::test(
            actor_cfg,
            Some(std::time::Duration::from_millis(1500)),
        ) {
            Ok(msg) => {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_handle.upgrade() {
                        ui.set_loading(false);
                        ui.set_status_text("Connection successful".into());
                        let _ = rfd::MessageDialog::new()
                            .set_level(rfd::MessageLevel::Info)
                            .set_title("Test Success")
                            .set_description(format!("Connection successful:\n{}", msg))
                            .show();
                    }
                });
            }
            Err(e) => {
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_handle.upgrade() {
                        ui.set_loading(false);
                        ui.set_status_text(format!("Connection failed: {}", e).into());
                        ui.set_test_enabled(false);
                        let _ = rfd::MessageDialog::new()
                            .set_level(rfd::MessageLevel::Error)
                            .set_title("Test Failure")
                            .set_description(format!("Connection failed:\n{}", e))
                            .show();
                    }
                });
            }
        }
    });
}
