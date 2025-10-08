use std::sync::{Arc, Mutex};

use fltk::prelude::*;

use crate::config_fltk::ConfigGui;

mod config_fltk;
mod regcfg;

use shared::{broker::api::types, log};

fn main() {
    log::setup_logging("debug", shared::log::LogType::Config);

    let operations = shared::operations::new_operations();

    // Our auths list, on Arc to share between threads
    let auths = Arc::new(Mutex::new(
        Vec::<shared::broker::api::types::Authenticator>::new(),
    ));
    // Las server used. To avoid re-querying the authenticators if the server hasn't changed
    // we store the last server in a Mutex<String> and only re-query if it changes
    let last_server = Arc::new(Mutex::new(String::new()));

    let app = fltk::app::App::default();
    let mut cfg = ConfigGui::new();
    // Add "Ignore certificate" and "Verify certificate" to choice_ssl_validation
    cfg.choice_ssl_validation
        .add_choice("Ignore certificate|Verify certificate");
    cfg.choice_ssl_validation.set_value(1); // Default to "Verify certificate"
    cfg.choice_ssl_validation.take_focus().unwrap();
    // Add DEBUG, INFO, WARNING, ERROR & CRITICAL to choice_log_level
    cfg.choice_log_level
        .add_choice("DEBUG|INFO|WARNING|ERROR|FATAL");
    cfg.choice_log_level.set_value(1); // Default to "INFO"

    // Default value for Authenticator is "Administration"
    cfg.choice_authenticator.add_choice("Administration");
    cfg.choice_authenticator.set_value(0); // Default to "Administration"

    cfg.input_uds_server.set_callback({
        let saved_auths = auths.clone();
        let last_server = last_server.clone();
        let mut choice_authenticator = cfg.choice_authenticator.clone();
        let choice_ssl = cfg.choice_ssl_validation.clone();
        // Set a callback on input_uds_server to validate the hostname

        move |s| {
            log::debug!("Using hostname: {}", s.value());
            let hostname = s.value().trim().to_string();
            if hostname.is_empty() {
                return;
            }
            // If the hostname hasn't changed, do nothing
            if *last_server.lock().unwrap() == hostname {
                log::debug!("Hostname hasn't changed, not re-querying authenticators");
                return;
            }
            *last_server.lock().unwrap() = hostname.clone();
            let cfg = regcfg::create_config(&hostname, choice_ssl.value() == 1);
            if let Ok(auths) = shared::broker::api::block::enumerate_authenticators(
                &cfg,
                Some(std::time::Duration::from_millis(800)),
            ) {
                // Store the authenticators in our Arc<Mutex<>>
                saved_auths.lock().unwrap().clear();
                saved_auths.lock().unwrap().extend(auths.clone());

                s.set_color(fltk::enums::Color::White);
                log::debug!(
                    "Authenticator enumeration successful, found {} authenticators",
                    auths.len()
                );
                let mut auth_names: Vec<String> = auths.iter().map(|a| a.label.clone()).collect();
                auth_names.sort();
                auth_names.dedup();
                choice_authenticator.clear();
                // Add "Administration" as the first choice, and select it
                choice_authenticator.add_choice("Administration");
                choice_authenticator.set_value(0);
                // Add all other authenticators
                for (i, name) in auth_names.iter().enumerate() {
                    choice_authenticator.add_choice(name);
                    if name == "Administration" {
                        choice_authenticator.set_value(i as i32);
                    }
                }
            } else {
                s.set_color(fltk::enums::Color::from_rgb(255, 100, 100)); // Light red
                log::debug!("Authenticator enumeration failed");
                choice_authenticator.clear();
                choice_authenticator.add_choice("Administration");
                choice_authenticator.set_value(0);
            }
            s.redraw();
            choice_authenticator.redraw();
        }
    });
    // Set the callback to register when the "Register" button is clicked
    cfg.button_register.set_callback({
        let auths = auths.clone();
        let cfg = cfg.clone();
        // Fail if we can't get at least one network interface
        let interface = operations
            .get_network_info()
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        let choice_ssl = cfg.choice_ssl_validation.clone();
        // Set a callback on input_uds_server to validate the hostname

        move |_| {
            let hostname = cfg.input_uds_server.value().trim().to_string();
            let selected_auth = if cfg.choice_authenticator.value() == 0 {
                "admin".to_string()
            } else {
                let auths = auths.lock().unwrap();
                if let Some(auth) = auths.get(cfg.choice_authenticator.value() as usize - 1) {
                    auth.auth.clone()
                } else {
                    "admin".to_string()
                }
            };
            let username = cfg.input_username.value().trim().to_string();
            let password = cfg.input_password.value().to_string();
            if hostname.is_empty() || username.is_empty() || password.is_empty() {
                fltk::dialog::alert_default("Hostname, username and password are required");
                return;
            }
            // Test that we can login to api
            let actor_cfg = regcfg::create_config(&hostname, choice_ssl.value() == 1);
            let token = match shared::broker::api::block::api_login(
                &actor_cfg,
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

            let log_level = match cfg.choice_log_level.value() {
                0 => types::LogLevel::Debug,
                1 => types::LogLevel::Info,
                2 => types::LogLevel::Warn,
                3 => types::LogLevel::Error,
                4 => types::LogLevel::Fatal,
                _ => types::LogLevel::Info,
            };
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
                command: types::RegisterCommandData {
                    pre_command: if cfg.input_preconnect_cmd.value().is_empty() {
                        None
                    } else {
                        Some(cfg.input_preconnect_cmd.value())
                    },
                    runonce_command: if cfg.input_runonce_cmd.value().is_empty() {
                        None
                    } else {
                        Some(cfg.input_runonce_cmd.value())
                    },
                    post_command: if cfg.input_postconfig_cmd.value().is_empty() {
                        None
                    } else {
                        Some(cfg.input_postconfig_cmd.value())
                    },
                },
                log_level,
                os: &os,
            };

            log::debug!(
                "Registering with hostname: {}, username: {}, ip: {}, mac: {}",
                reg_auth.hostname,
                reg_auth.username,
                reg_auth.ip,
                reg_auth.mac
            );

            match shared::broker::api::block::register(&actor_cfg, &reg_auth, &token) {
                Ok(secret) => {
                    fltk::dialog::message_default(&format!(
                        "Registration successful!\n\nSecret: {}\n\nPlease save this secret securely, as it will not be shown again.",
                        secret
                    ));
                    log::info!("Registration successful, got secret: {}", secret);
                }
                Err(e) => {
                    fltk::dialog::alert_default(&format!("Registration failed: {}", e));
                    log::error!("Registration failed: {}", e);
                }
            }


        }
    });

    cfg.win.center_screen();
    app.run().unwrap();
}
