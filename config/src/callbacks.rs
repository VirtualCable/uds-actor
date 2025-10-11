use std::sync::{Arc, Mutex};

use fltk::prelude::*;

use shared::{broker::api::types, config, log, operations::NetworkInterface};

use crate::{config_fltk::ConfigGui, regcfg};

pub fn uds_server_changed(
    cfg_window: &ConfigGui,
    saved_auths: Arc<Mutex<Vec<shared::broker::api::types::Authenticator>>>,
) {
    // If udsserver is empty, do nothing
    if cfg_window.input_uds_server.value().trim().is_empty() {
        return;
    }
    let mut cfg_window = cfg_window.clone();
    let hostname = cfg_window.input_uds_server.value().trim().to_string();
    let actor_cfg =
        regcfg::broker_api_config(&hostname, cfg_window.choice_ssl_validation.value() == 1);
    if let Ok(mut auths) = shared::broker::api::block::enumerate_authenticators(
        actor_cfg,
        Some(std::time::Duration::from_millis(800)),
    ) {
        // Sort auths by name before storing
        auths.sort_by(|a, b| a.name.cmp(&b.name));

        // Store the authenticators in our Arc<Mutex<>>
        saved_auths.lock().unwrap().clear();
        saved_auths.lock().unwrap().extend(auths.clone());

        cfg_window
            .input_uds_server
            .set_color(fltk::enums::Color::White);
        log::debug!(
            "Authenticator enumeration successful, found {} authenticators",
            auths.len()
        );
        let mut auth_names: Vec<String> = auths.iter().map(|a| a.name.clone()).collect();
        auth_names.sort();
        auth_names.dedup();

        // Add "Administration" as the first choice, and select it
        cfg_window.choice_authenticator.add_choice("Administration");
        cfg_window.choice_authenticator.set_value(0);
        // Add all other authenticators
        for (i, name) in auth_names.iter().enumerate() {
            cfg_window.choice_authenticator.add_choice(name);
            if name == "Administration" {
                cfg_window.choice_authenticator.set_value(i as i32);
            }
        }
    } else {
        cfg_window
            .input_uds_server
            .set_color(fltk::enums::Color::from_rgb(255, 100, 100)); // Light red
        log::debug!("Authenticator enumeration failed");
        cfg_window.choice_authenticator.clear();
        cfg_window.choice_authenticator.add_choice("Administration");
        cfg_window.choice_authenticator.set_value(0);
    }
    cfg_window.input_uds_server.redraw();
    cfg_window.choice_authenticator.redraw();
}

/// Callback for the "Register" button
/// - Validate fields
/// - Login to API
/// - Register the actor
pub fn btn_register_clicked(
    cfg_window: &ConfigGui,
    auths: Arc<Mutex<Vec<shared::broker::api::types::Authenticator>>>,
    operations: Arc<dyn shared::operations::Operations>,
    interface: &NetworkInterface,
) {
    let hostname = cfg_window.input_uds_server.value().trim().to_string();
    let selected_auth = if cfg_window.choice_authenticator.value() == 0 {
        "admin".to_string()
    } else {
        let auths = auths.lock().unwrap();
        if let Some(auth) = auths.get(cfg_window.choice_authenticator.value() as usize - 1) {
            auth.name.clone()
        } else {
            "admin".to_string()
        }
    };
    let username = cfg_window.input_username.value().trim().to_string();
    let password = cfg_window.input_password.value().to_string();
    if hostname.is_empty() || username.is_empty() || password.is_empty() {
        fltk::dialog::alert_default("Hostname, username and password are required");
        return;
    }
    // Test that we can login to api
    let actor_cfg =
        regcfg::broker_api_config(&hostname, cfg_window.choice_ssl_validation.value() == 1);
    let token = match shared::broker::api::block::api_login(
        actor_cfg.clone(),
        &selected_auth,
        &username,
        &password,
    ) {
        Ok(token) => {
            log::debug!("Login successful, got token: {}", token);
            token
        }
        Err(e) => {
            fltk::dialog::alert_default(&format!("Login failed: {}", e));
            return;
        }
    };

    // Username on registry has @authname at the end
    let username = username + "@" + &selected_auth;

    let log_level: types::LogLevel = (cfg_window.choice_log_level.value() as u8).min(4).into();

    let os = operations.get_os_version().unwrap_or_default();
    let computer_name = operations.get_computer_name().unwrap_or_default();
    // Get selected index of choice_authenticator
    let reg_auth = types::RegisterRequest {
        version: shared::consts::VERSION,
        build: shared::consts::BUILD,
        hostname: computer_name.as_str(),
        username: username.as_str(),
        ip: interface.ip_addr.as_str(),
        mac: interface.mac.as_str(),
        commands: types::RegisterCommands {
            pre_command: if cfg_window.input_preconnect_cmd.value().is_empty() {
                None
            } else {
                Some(cfg_window.input_preconnect_cmd.value())
            },
            runonce_command: if cfg_window.input_runonce_cmd.value().is_empty() {
                None
            } else {
                Some(cfg_window.input_runonce_cmd.value())
            },
            post_command: if cfg_window.input_postconfig_cmd.value().is_empty() {
                None
            } else {
                Some(cfg_window.input_postconfig_cmd.value())
            },
        },
        log_level: log_level.into(),
        os: &os,
    };

    log::debug!(
        "Registering with hostname: {}, username: {}, ip: {}, mac: {}",
        reg_auth.hostname,
        reg_auth.username,
        reg_auth.ip,
        reg_auth.mac
    );

    match shared::broker::api::block::register(actor_cfg, &reg_auth, &token) {
        Ok(master_token) => {
            log::debug!("Registration successful, got token: {}", master_token);
            // Save config to file
            let final_cfg = config::ActorConfiguration {
                broker_url: format!("https://{}/uds/rest/", hostname),
                verify_ssl: cfg_window.choice_ssl_validation.value() == 1,
                actor_type: config::ActorType::Managed,
                master_token: Some(master_token),
                own_token: None,
                restrict_net: None,
                pre_command: reg_auth.commands.pre_command,
                runonce_command: reg_auth.commands.runonce_command,
                post_command: reg_auth.commands.post_command,
                log_level: log_level.into(),
                config: None,
                data: None,
            };
            let mut config_storage = config::new_config_storage();
            if let Err(e) = config_storage.save_config(&final_cfg) {
                fltk::dialog::alert_default(&format!("Failed to save config: {}", e));
                log::error!("Failed to save config: {}", e);
            } else {
                fltk::dialog::message_default("Registration successful!\n");
                let mut btn_test = cfg_window.button_test.clone();
                btn_test.activate(); // Enable test button
                log::debug!("Config saved successfully");
            }
        }
        Err(e) => {
            fltk::dialog::alert_default(&format!("Registration failed: {}", e));
            log::error!("Registration failed: {}", e);
        }
    }
}

pub fn btn_test_clicked() {
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
        fltk::dialog::alert_default("Register with UDS before testing connection");
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
        }
    }
}
