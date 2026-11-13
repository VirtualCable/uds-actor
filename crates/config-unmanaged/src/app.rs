/*!
Author: Andres Schumann, aschumann at virtualcable dot es
*/
use eframe::egui::{self, ComboBox, Context, Id, TextEdit, TextureHandle, Ui, ViewportCommand};
use gui_kit::status::{Level, Status};
use gui_kit::task::{LOST, Task};
use gui_kit::{logo, theme, widgets};

use crate::callbacks;

pub const LOG_LEVELS: [&str; 5] = ["DEBUG", "INFO", "WARNING", "ERROR", "FATAL"];
const SSL_MODES: [&str; 2] = ["Ignore certificate", "Verify certificate"];
const SAVE_WIDTH: f32 = 180.0;

pub struct Fields {
    pub server_host: String,
    pub verify_ssl: bool,
    pub service_token: String,
    pub net_restriction: String,
    pub log_level: usize,
    pub ssl_ciphers: String,
}

impl Default for Fields {
    fn default() -> Self {
        Self {
            server_host: String::new(),
            verify_ssl: true,
            service_token: String::new(),
            net_restriction: String::new(),
            log_level: 1,
            ssl_ciphers: String::new(),
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Tab {
    #[default]
    General,
    Advanced,
    About,
}

pub struct ConfigApp {
    fields: Fields,
    test_enabled: bool,
    status: Status,
    tab: Tab,
    logo: TextureHandle,
    test_task: Option<Task<Result<String, String>>>,
}

impl ConfigApp {
    pub fn new(ctx: &Context, fields: Fields, test_enabled: bool) -> Self {
        Self {
            fields,
            test_enabled,
            status: Status::default(),
            tab: Tab::default(),
            logo: logo::texture(ctx),
            test_task: None,
        }
    }

    fn busy(&self) -> bool {
        self.test_task.is_some()
    }

    fn poll_test(&mut self) {
        let finished = self
            .test_task
            .as_ref()
            .and_then(|task| task.poll_or(|| Err(LOST.to_string())));
        if let Some(result) = finished {
            self.test_task = None;
            match result {
                Ok(text) => self.status = Status::new(Level::Ready, text),
                Err(text) => {
                    self.test_enabled = false;
                    self.status = Status::new(Level::Error, text);
                }
            }
        }
    }

    fn save(&mut self) {
        let host = self.fields.server_host.trim().to_string();
        if host.is_empty() {
            self.status = Status::new(Level::Error, "Hostname is required");
            return;
        }
        let token = self.fields.service_token.trim().to_string();
        let net = self.fields.net_restriction.trim().to_string();
        let ciphers = self.fields.ssl_ciphers.trim().to_string();

        self.status = match callbacks::save_unmanaged(
            &host,
            self.fields.verify_ssl,
            &token,
            &net,
            (self.fields.log_level as u8).min(4).into(),
            &ciphers,
        ) {
            Ok(()) => {
                self.test_enabled = true;
                Status::new(Level::Ready, "Configuration saved successfully!")
            }
            Err(error) => Status::new(Level::Error, format!("Failed to save config: {error}")),
        };
    }

    fn test_connection(&mut self, ctx: &Context) {
        match callbacks::registered_config() {
            Err(error) => self.status = Status::new(Level::Error, error),
            Ok(cfg) => {
                self.status = Status::new(Level::Busy, "Testing connection...");
                self.test_task = Some(Task::spawn(ctx, move || callbacks::test_connection(cfg)));
            }
        }
    }

    fn general_tab(&mut self, ui: &mut Ui) {
        let mut ssl_mode = usize::from(self.fields.verify_ssl);
        widgets::row(ui, "SSL Validation:", |ui, label| {
            index_combo(ui, "ssl-mode", label, &mut ssl_mode, &SSL_MODES)
        });
        self.fields.verify_ssl = ssl_mode == 1;

        widgets::row(ui, "UDS Broker Host:", |ui, label| {
            let width = ui.available_width();
            ui.add(
                TextEdit::singleline(&mut self.fields.server_host)
                    .hint_text("broker.example.com")
                    .desired_width(width),
            )
            .labelled_by(label)
        });

        widgets::row(ui, "Service Token:", |ui, label| {
            let width = ui.available_width();
            ui.add(TextEdit::singleline(&mut self.fields.service_token).desired_width(width))
                .labelled_by(label)
        });

        widgets::row(ui, "Net Restriction:", |ui, label| {
            let width = ui.available_width();
            ui.add(
                TextEdit::singleline(&mut self.fields.net_restriction)
                    .hint_text("e.g. 192.168.1.0/24")
                    .desired_width(width),
            )
            .labelled_by(label)
        });

        widgets::row(ui, "Log level:", |ui, label| {
            index_combo(
                ui,
                "log-level",
                label,
                &mut self.fields.log_level,
                &LOG_LEVELS,
            )
        });
    }

    fn advanced_tab(&mut self, ui: &mut Ui) {
        widgets::section_title(ui, "Custom SSL Cipher Suites:");
        ui.add(
            TextEdit::multiline(&mut self.fields.ssl_ciphers)
                .desired_rows(7)
                .desired_width(f32::INFINITY),
        );
        widgets::hint(
            ui,
            "Leave empty to use secure defaults. Separated by colon (:).",
        );
        if !callbacks::ciphers_look_valid(&self.fields.ssl_ciphers) {
            widgets::warning(ui, "Warning: Some ciphers might be invalid or unsupported.");
        }
    }

    fn tabs(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            for (tab, title) in [
                (Tab::General, "General"),
                (Tab::Advanced, "Advanced"),
                (Tab::About, "About"),
            ] {
                ui.selectable_value(&mut self.tab, tab, title);
            }
        });
        ui.separator();
        match self.tab {
            Tab::General => self.general_tab(ui),
            Tab::Advanced => self.advanced_tab(ui),
            Tab::About => widgets::about(ui, "UDS Actor (Unmanaged)", shared::consts::VERSION),
        }
    }

    fn buttons(&mut self, ui: &mut Ui, ctx: &Context) {
        let busy = self.busy();
        let mut save = false;
        let mut test = false;
        let mut close = false;
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                save = ui
                    .add_enabled(
                        !busy,
                        widgets::primary_button("Save Configuration", SAVE_WIDTH),
                    )
                    .clicked();
                test = ui
                    .add_enabled(
                        self.test_enabled && !busy,
                        widgets::button("Test", theme::BUTTON_WIDTH),
                    )
                    .clicked();
                close = ui
                    .add(widgets::button("Close", theme::BUTTON_WIDTH))
                    .clicked();
            });
        });
        if save {
            self.save();
        }
        if test {
            self.test_connection(ctx);
        }
        if close {
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    pub fn body(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        self.poll_test();
        widgets::header(ui, &self.logo, "UDS Actor (Unmanaged)");
        self.tabs(ui);
        widgets::status_line(ui, &self.status);
        self.buttons(ui, &ctx);
    }
}

impl eframe::App for ConfigApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        theme::apply(ui.ctx());
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).inner_margin(theme::panel_margin()))
            .show(ui, |ui| self.body(ui));
    }
}

fn index_combo<S: AsRef<str>>(
    ui: &mut Ui,
    id: &str,
    label: Id,
    selected: &mut usize,
    options: &[S],
) {
    ComboBox::from_id_salt(id)
        .width(ui.available_width())
        .show_index(ui, selected, options.len(), |index| options[index].as_ref())
        .labelled_by(label);
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    fn harness() -> Harness<'static, ConfigApp> {
        let ctx = Context::default();
        let app = ConfigApp::new(&ctx, Fields::default(), false);
        Harness::new_ui_state(|ui, app: &mut ConfigApp| app.body(ui), app)
    }

    #[test]
    fn about_tab_shows_the_product_copyright() {
        let mut harness = harness();
        harness.get_by_label("About").click();
        harness.run();
        harness.get_by_label("Copyright (c) Virtual Cable S.L.U.");
    }

    #[test]
    fn save_without_host_reports_it() {
        let mut harness = harness();
        harness.get_by_label("Save Configuration").click();
        harness.run();
        harness.get_by_label("Hostname is required");
    }
}
