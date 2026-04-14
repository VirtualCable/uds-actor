use slint::ComponentHandle;
use std::sync::{Arc, Mutex};

use shared::{
    broker::api::{block, types},
    config, log,
    system::NetworkInterface,
};

use crate::AppWindow;
use crate::regcfg;

pub fn uds_server_changed(
    ui: &AppWindow,
    saved_auths: Arc<Mutex<Vec<shared::broker::api::types::Authenticator>>>,
) {
    let hostname = ui.get_server_host().trim().to_string();
    if hostname.is_empty() {
        return;
    }

    let verify_ssl = ui.get_verify_ssl();
    let ui_handle = ui.as_weak();

    ui.set_loading(true);
    ui.set_status_text("Querying authenticators...".into());

    std::thread::spawn(move || {
        let actor_cfg = regcfg::broker_api_config(&hostname, verify_ssl, "");
        match block::enumerate_authenticators(
            actor_cfg,
            Some(std::time::Duration::from_millis(1500)),
        ) {
            Ok(mut auths) => {
                // Sort auths by name before storing
                auths.sort_by(|a, b| a.name.cmp(&b.name));

                // Store the authenticators in our Arc<Mutex<>>
                {
                    let mut lock = saved_auths.lock().unwrap();
                    lock.clear();
                    lock.extend(auths.clone());
                }

                let auth_names: Vec<String> = auths.iter().map(|a| a.name.clone()).collect();

                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_handle.upgrade() {
                        ui.set_loading(false);
                        ui.set_status_text("Ready".into());

                        let mut models = vec!["Administration".to_string()];
                        models.extend(auth_names);

                        let model = std::rc::Rc::new(slint::VecModel::from(
                            models
                                .into_iter()
                                .map(|s| s.into())
                                .collect::<Vec<slint::SharedString>>(),
                        ));
                        ui.set_authenticators(model.into());
                        ui.set_active_authenticator(0);
                    }
                });
            }
            Err(e) => {
                log::warn!("Authenticator enumeration failed: {}", e);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_handle.upgrade() {
                        ui.set_loading(false);
                        ui.set_status_text(format!("Error: {}", e).into());

                        let model =
                            std::rc::Rc::new(slint::VecModel::from(vec!["Administration".into()]));
                        ui.set_authenticators(model.into());
                        ui.set_active_authenticator(0);
                    }
                });
            }
        };
    });
}

pub fn btn_register_clicked(
    ui: &AppWindow,
    auths: Arc<Mutex<Vec<shared::broker::api::types::Authenticator>>>,
    operations: Arc<dyn shared::system::System>,
    interface: &NetworkInterface,
) {
    let hostname = ui.get_server_host().trim().to_string();
    let auth_idx = ui.get_active_authenticator();

    let selected_auth = if auth_idx == 0 {
        "admin".to_string()
    } else {
        let auths_lock = auths.lock().unwrap();
        if let Some(auth) = auths_lock.get(auth_idx as usize - 1) {
            auth.name.clone()
        } else {
            "admin".to_string()
        }
    };

    let username = ui.get_username().trim().to_string();
    let password = ui.get_password_val().to_string();
    let ciphers = ui.get_ssl_ciphers().trim().to_string();

    if hostname.is_empty() || username.is_empty() || password.is_empty() {
        std::thread::spawn(|| {
            let _ = rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Error)
                .set_title("Validation Error")
                .set_description("Hostname, username and password are required")
                .show();
        });
        return;
    }

    ui.set_loading(true);
    ui.set_status_text("Registering...".into());

    let actor_cfg = regcfg::broker_api_config(&hostname, ui.get_verify_ssl(), &ciphers);
    let ui_handle = ui.as_weak();
    let ops = operations.clone();
    let iface = interface.clone();

    std::thread::spawn(move || {
        // Login
        let token_res = shared::broker::api::block::api_login(
            actor_cfg.clone(),
            &selected_auth,
            &username,
            &password,
        );

        match token_res {
            Ok(token) => {
                log::debug!("Login successful, got token");

                let username_full = format!("{}@{}", username, selected_auth);
                let log_level_idx = ui_handle
                    .upgrade()
                    .map(|ui| ui.get_active_log_level())
                    .unwrap_or(1);
                let log_level: types::LogLevel = (log_level_idx as u8).min(4).into();

                let os = ops.get_os_version().unwrap_or_default();
                let computer_name = ops.get_computer_name().unwrap_or_default();

                let (pre_cmd, run_cmd, post_cmd) = if let Some(ui) = ui_handle.upgrade() {
                    (
                        ui.get_preconnect_cmd().trim().to_string(),
                        ui.get_runonce_cmd().trim().to_string(),
                        ui.get_postconfig_cmd().trim().to_string(),
                    )
                } else {
                    (String::new(), String::new(), String::new())
                };

                let reg_auth = types::RegisterRequest {
                    version: shared::consts::VERSION,
                    build: shared::consts::BUILD,
                    hostname: &computer_name,
                    username: &username_full,
                    ip: &iface.ip_addr,
                    mac: &iface.mac,
                    commands: types::RegisterCommands {
                        pre_command: if pre_cmd.is_empty() {
                            None
                        } else {
                            Some(pre_cmd.clone())
                        },
                        runonce_command: if run_cmd.is_empty() {
                            None
                        } else {
                            Some(run_cmd.clone())
                        },
                        post_command: if post_cmd.is_empty() {
                            None
                        } else {
                            Some(post_cmd.clone())
                        },
                    },
                    log_level: log_level.into(),
                    os: &os,
                };

                match shared::broker::api::block::register(actor_cfg.clone(), &reg_auth, &token) {
                    Ok(master_token) => {
                        log::debug!("Registration successful");

                        let final_cfg = config::ActorConfiguration {
                            broker_url: format!("https://{}/uds/rest/", hostname),
                            verify_ssl: actor_cfg.verify_ssl,
                            actor_type: config::ActorType::Managed,
                            master_token: Some(master_token),
                            own_token: None,
                            restrict_net: None,
                            pre_command: reg_auth.commands.pre_command,
                            runonce_command: reg_auth.commands.runonce_command,
                            post_command: reg_auth.commands.post_command,
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
                        let res = config_storage.save_config(&final_cfg);

                        let res_err = res.as_ref().err().map(|e| e.to_string());
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_handle.upgrade() {
                                ui.set_loading(false);
                                if let Some(err) = res_err {
                                    ui.set_status_text(
                                        format!("Failed to save config: {}", err).into(),
                                    );
                                } else {
                                    ui.set_status_text("Registration successful".into());
                                    ui.set_test_enabled(true);
                                }
                            }
                        });

                        // Show dialog from background thread
                        match res {
                            Err(e) => {
                                let _ = rfd::MessageDialog::new()
                                    .set_level(rfd::MessageLevel::Error)
                                    .set_title("Save Error")
                                    .set_description(format!("Failed to save config: {}", e))
                                    .show();
                            }
                            Ok(_) => {
                                let _ = rfd::MessageDialog::new()
                                    .set_level(rfd::MessageLevel::Info)
                                    .set_title("Success")
                                    .set_description("Registration successful!")
                                    .show();
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Registration failed: {}", e);
                        let err_msg = e.to_string();
                        let err_msg_ui = err_msg.clone();
                        let _ = slint::invoke_from_event_loop(move || {
                            if let Some(ui) = ui_handle.upgrade() {
                                ui.set_loading(false);
                                ui.set_status_text(format!("Registration failed: {}", err_msg_ui).into());
                            }
                        });
                        let _ = rfd::MessageDialog::new()
                            .set_level(rfd::MessageLevel::Error)
                            .set_title("Registration Error")
                            .set_description(format!("Registration failed: {}", err_msg))
                            .show();
                    }
                }
            }
            Err(e) => {
                log::error!("Login failed: {}", e);
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_handle.upgrade() {
                        ui.set_loading(false);
                        ui.set_status_text(format!("Login failed: {}", e).into());
                    }
                });
                let _ = rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_title("Login Error")
                    .set_description(
                        "Login failed. Check credentials and server connectivity.",
                    )
                    .show();
            }
        }
    });
}

pub fn btn_test_clicked(ui: &AppWindow) {
    log::debug!("Test connection button clicked");
    let cfg = config::new_config_storage().load_config();
    if let Err(err) = cfg {
        std::thread::spawn(move || {
            let _ = rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Error)
                .set_title("Config Error")
                .set_description(format!("Failed to load existing config: {}", err))
                .show();
        });
        return;
    }

    let actor_cfg = cfg.unwrap();
    if actor_cfg.broker_url.is_empty() || actor_cfg.token().is_empty() {
        std::thread::spawn(|| {
            let _ = rfd::MessageDialog::new()
                .set_level(rfd::MessageLevel::Warning)
                .set_title("Action Required")
                .set_description("Please register with UDS before testing the connection")
                .show();
        });
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
                    }
                });
                let _ = rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Info)
                    .set_title("Test Success")
                    .set_description(format!("Connection successful:\n{}", msg))
                    .show();
            }
            Err(e) => {
                let err_msg = e.to_string();
                let err_msg_ui = err_msg.clone();
                let _ = slint::invoke_from_event_loop(move || {
                    if let Some(ui) = ui_handle.upgrade() {
                        ui.set_loading(false);
                        ui.set_status_text(format!("Connection failed: {}", err_msg_ui).into());
                    }
                });
                let _ = rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_title("Test Failure")
                    .set_description(format!("Connection failed:\n{}", err_msg))
                    .show();
            }
        }
    });
}
