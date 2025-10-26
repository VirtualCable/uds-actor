#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::sync::{Arc, Mutex};

use fltk::{dialog::NativeFileChooser, enums::CallbackTrigger, prelude::*};

use crate::config_fltk::ConfigGui;

mod callbacks;
mod config_fltk;
mod regcfg;

use shared::log;

fn main() {
    log::setup_logging("debug", shared::log::LogType::Config);

    let operations = shared::operations::new_operations();

    // On debug builds, skip the admin check
    #[cfg(not(debug_assertions))]
    {
        if operations.check_permissions().is_err() {
            fltk::dialog::alert_default("This program must be run with administrator privileges");
            std::process::exit(1);
        }
    }

    // Our auths list, on Arc to share between threads
    let auths = Arc::new(Mutex::new(
        Vec::<shared::broker::api::types::Authenticator>::new(),
    ));
    // Las server used. To avoid re-querying the authenticators if the server hasn't changed
    // we store the last server in a Mutex<String> and only re-query if it changes
    let last_server = Arc::new(Mutex::new(String::new()));

    let app = fltk::app::App::default();
    let mut cfg_window = ConfigGui::new();

    cfg_window.button_test.deactivate(); // Disabled until we have a valid config

    // Eat "escape" key presses to avoid closing the window
    cfg_window.win.set_callback({
        move |_| {
            log::debug!("Window callback triggered: event={:?}", fltk::app::event());
            if fltk::app::event() == fltk::enums::Event::Shortcut
                && fltk::app::event_key() == fltk::enums::Key::Escape
            {
                // Just eat the event
                log::debug!("Escape pressed, ignoring");
            } else {
                fltk::app::quit();
            }
        }
    });

    // Add "Ignore certificate" and "Verify certificate" to choice_ssl_validation
    cfg_window
        .choice_ssl_validation
        .add_choice("Ignore certificate|Verify certificate");
    cfg_window.choice_ssl_validation.set_value(1); // Default to "Verify certificate"
    cfg_window.choice_ssl_validation.take_focus().unwrap();
    // Add DEBUG, INFO, WARNING, ERROR & CRITICAL to choice_log_level
    cfg_window
        .choice_log_level
        .add_choice("DEBUG|INFO|WARNING|ERROR|FATAL");
    cfg_window.choice_log_level.set_value(1); // Default to "INFO"

    // Default value for Authenticator is "Administration"
    cfg_window.choice_authenticator.add_choice("Administration");
    cfg_window.choice_authenticator.set_value(0); // Default to "Administration"

    cfg_window
        .input_uds_server
        .set_trigger(CallbackTrigger::ReleaseAlways);
    cfg_window.input_uds_server.set_callback({
        let saved_auths = auths.clone();
        let cfg_window = cfg_window.clone();
        // Set a callback on input_uds_server to validate the hostname

        move |s| {
            log::debug!("Using UDS Server: {}", s.value());
            let uds_server = s.value().trim().to_string();
            if uds_server.is_empty() {
                return;
            }
            // If the UDS Server + ssl hasn't changed, do nothing
            if *last_server.lock().unwrap()
                == uds_server.clone()
                    + cfg_window
                        .choice_ssl_validation
                        .value()
                        .to_string()
                        .as_str()
            {
                log::debug!("UDS Server hasn't changed, not re-querying authenticators");
                return;
            }
            *last_server.lock().unwrap() = uds_server.clone()
                + cfg_window
                    .choice_ssl_validation
                    .value()
                    .to_string()
                    .as_str();

            callbacks::uds_server_changed(&cfg_window, saved_auths.clone());
        }
    });
    // Set the callback to register when the "Register" button is clicked
    cfg_window.button_register.set_callback({
        let auths = auths.clone();
        let cfg_window = cfg_window.clone();
        // Fail if we can't get at least one network interface
        let interface = operations
            .get_first_network_interface()
            .unwrap_or_else(|e| {
                log::error!("No network interfaces found: {}", e);
                fltk::dialog::alert_default("No network interfaces found, cannot continue");
                fltk::app::quit();
                std::process::exit(1);
            });

        move |_| {
            callbacks::btn_register_clicked(
                &cfg_window,
                auths.clone(),
                operations.clone(),
                &interface,
            );
        }
    });

    cfg_window.button_test.set_callback({
        move |_| {
            callbacks::btn_test_clicked();
        }
    });

    // Set the close button to quit the app
    cfg_window.button_close.set_callback({
        move |_| {
            log::debug!("Close button clicked, quitting");
            fltk::app::quit();
        }
    });

    // Setup buttons for browsing files, postconfig_cmd
    cfg_window.browse_postconfig_cmd.set_callback({
        let mut input = cfg_window.input_postconfig_cmd.clone();
        move |_| {
            let mut dlg = NativeFileChooser::new(fltk::dialog::FileDialogType::BrowseFile);
            dlg.show();
            if let Some(path) = dlg.filename().to_str()
                && !path.is_empty()
            {
                input.set_value(path);
            }
        }
    });

    // preconnect_cmd
    cfg_window.browse_preconnect_cmd.set_callback({
        let mut input = cfg_window.input_preconnect_cmd.clone();
        move |_| {
            let mut dlg = NativeFileChooser::new(fltk::dialog::FileDialogType::BrowseFile);
            dlg.show();
            if let Some(path) = dlg.filename().to_str()
                && !path.is_empty()
            {
                input.set_value(path);
            }
        }
    });

    // runonce_cmd
    cfg_window.browse_runonce_cmd.set_callback({
        let mut input = cfg_window.input_runonce_cmd.clone();
        move |_| {
            let mut dlg = NativeFileChooser::new(fltk::dialog::FileDialogType::BrowseFile);
            dlg.show();
            if let Some(path) = dlg.filename().to_str()
                && !path.is_empty()
            {
                input.set_value(path);
            }
        }
    });

    // Fill the fields from existing config
    regcfg::fill_window_fields(&mut cfg_window);
    callbacks::uds_server_changed(&cfg_window, auths.clone());

    cfg_window.win.center_screen();
    app.run().unwrap();
}
