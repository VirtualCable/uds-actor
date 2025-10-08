use fltk::prelude::MenuExt;

use crate::config_fltk::ConfigGui;

mod config_fltk;

fn main() {
    let app = fltk::app::App::default();
    let mut cfg = ConfigGui::new();
    // Add "Ignore certificate" and "Verify certificate" to choice_ssl_validation
    cfg.choice_ssl_validation.add_choice("Ignore certificate|Verify certificate");
    cfg.choice_ssl_validation.set_value(1); // Default to "Verify certificate"
    app.run().unwrap();
}
