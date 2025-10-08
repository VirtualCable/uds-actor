use fltk::prelude::*;

use crate::config_fltk::ConfigGui;

mod config_fltk;
mod regcfg;

use shared::log;

fn main() {
    log::setup_logging("debug", shared::log::LogType::Config);
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
    cfg.choice_authenticator
        .add_choice("Administration");
    cfg.choice_authenticator.set_value(0); // Default to "Administration"

    let choice_ssl = cfg.choice_ssl_validation.clone();
    let mut choice_authenticator = cfg.choice_authenticator.clone();
    // Set a callback on input_uds_server to validate the hostname
    cfg.input_uds_server.set_callback(move |s| {
        log::debug!("Validating hostname: {}", s.value());
        let hostname = s.value();
        let cfg = regcfg::create_config(&hostname, choice_ssl.value() == 1);
        if let Ok(auths) = shared::broker::api::block::enumerate_authenticators(cfg) {
            s.set_color(fltk::enums::Color::White);
            log::debug!("Authenticator enumeration successful, found {} authenticators", auths.len());
            let mut auth_names: Vec<String> = auths.iter().map(|a| a.label.clone()).collect();
            auth_names.sort();
            auth_names.dedup();
            choice_authenticator.clear();
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
    });
    cfg.win.center_screen();
    app.run().unwrap();
}
