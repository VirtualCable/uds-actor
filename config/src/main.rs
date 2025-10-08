use fltk::prelude::MenuExt;

use crate::config_fltk::ConfigGui;

mod config_fltk;

fn main() {
    let app = fltk::app::App::default();
    let mut cfg = ConfigGui::new();
    // Add "Ignore certificate" and "Verify certificate" to choice_ssl_validation
    cfg.choice_ssl_validation.add_choice("Ignore certificate|Verify certificate");
    cfg.choice_ssl_validation.set_value(1); // Default to "Verify certificate"
    // Add DEBUG, INFO, WARNING, ERROR & CRITICAL to choice_log_level
    cfg.choice_log_level.add_choice("DEBUG|INFO|WARNING|ERROR|FATAL");
    cfg.choice_log_level.set_value(1); // Default to "INFO"
    app.run().unwrap();
}
