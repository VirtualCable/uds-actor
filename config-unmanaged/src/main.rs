use fltk::prelude::*;

use crate::config_unmanaged_fltk::ConfigGui;

mod callbacks;
mod config_unmanaged_fltk;
mod regcfg;

use shared::log;

fn main() {
    log::setup_logging("debug", shared::log::LogType::Config);

    // On debug builds, skip the admin check
    #[cfg(not(debug_assertions))]
    {
        let operations = shared::operations::new_operations();
        if operations.check_permissions().is_err() {
            fltk::dialog::alert_default("This program must be run with administrator privileges");
            std::process::exit(1);
        }
    }

    let app = fltk::app::App::default();
    let mut cfg_window = ConfigGui::new();

    // Disable button_test until we have a valid config
    cfg_window.button_test.deactivate();

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

    // Set the callback to register when the "Save" button is clicked
    cfg_window.button_save.set_callback({
        let cfg_window = cfg_window.clone();
        // Set a callback on input_uds_server to validate the hostname

        move |_| {
            callbacks::bnt_save_clicked(&cfg_window);
        }
    });

    // Set the close button to quit the app
    cfg_window.button_close.set_callback({
        move |_| {
            log::debug!("Close button clicked, quitting");
            fltk::app::quit();
        }
    });

    cfg_window.button_test.set_callback({
        let cfg_window = cfg_window.clone();
        move |_| {
            callbacks::btn_test_clicked(&cfg_window);
        }
    });

    // Fill the fields from existing config
    regcfg::fill_window_fields(&mut cfg_window);

    cfg_window.win.center_screen();
    app.run().unwrap();
}
