/*!
Author: Andres Schumann, aschumann at virtualcable dot es
*/
use std::sync::Arc;

use eframe::egui::{self, ComboBox, Context, Id, TextEdit, TextureHandle, Ui, ViewportCommand};
use gui_kit::status::{Level, Status};
use gui_kit::task::{LOST, Task};
use gui_kit::{logo, theme, widgets};
use shared::broker::api::types::Authenticator;
use shared::log;
use shared::system::{NetworkInterface, System};

use crate::callbacks::{self, Action, RegistrationInput};

pub const LOG_LEVELS: [&str; 5] = ["DEBUG", "INFO", "WARNING", "ERROR", "FATAL"];
const SSL_MODES: [&str; 2] = ["Ignore certificate", "Verify certificate"];
const ADMINISTRATION: &str = "Administration";
const BROWSE_WIDTH: f32 = 80.0;
const REGISTER_WIDTH: f32 = 150.0;

pub struct Fields {
    pub server_host: String,
    pub verify_ssl: bool,
    pub username: String,
    pub password: String,
    pub log_level: usize,
    pub preconnect_cmd: String,
    pub runonce_cmd: String,
    pub postconfig_cmd: String,
    pub ssl_ciphers: String,
}

impl Default for Fields {
    fn default() -> Self {
        Self {
            server_host: String::new(),
            verify_ssl: true,
            username: String::new(),
            password: String::new(),
            log_level: 1,
            preconnect_cmd: String::new(),
            runonce_cmd: String::new(),
            postconfig_cmd: String::new(),
            ssl_ciphers: String::new(),
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum Tab {
    #[default]
    General,
    Commands,
    Advanced,
    About,
}

pub struct ConfigApp {
    fields: Fields,
    authenticators: Vec<Authenticator>,
    auth_index: usize,
    test_enabled: bool,
    status: Status,
    tab: Tab,
    logo: TextureHandle,
    operations: Arc<dyn System>,
    auths_task: Option<Task<Result<Vec<Authenticator>, String>>>,
    action: Option<(Action, Task<Result<String, String>>)>,
}

impl ConfigApp {
    pub fn new(
        ctx: &Context,
        operations: Arc<dyn System>,
        fields: Fields,
        test_enabled: bool,
    ) -> Self {
        let mut app = Self {
            fields,
            authenticators: Vec::new(),
            auth_index: 0,
            test_enabled,
            status: Status::default(),
            tab: Tab::default(),
            logo: logo::texture(ctx),
            operations,
            auths_task: None,
            action: None,
        };
        app.refresh_authenticators(ctx);
        app
    }

    fn busy(&self) -> bool {
        self.auths_task.is_some() || self.action.is_some()
    }

    fn auth_names(&self) -> Vec<String> {
        std::iter::once(ADMINISTRATION.to_string())
            .chain(self.authenticators.iter().map(|auth| auth.name.clone()))
            .collect()
    }

    fn selected_auth(&self) -> String {
        match self
            .auth_index
            .checked_sub(1)
            .and_then(|index| self.authenticators.get(index))
        {
            Some(auth) => auth.name.clone(),
            None => "admin".to_string(),
        }
    }

    fn poll_tasks(&mut self) {
        let enumerated = self
            .auths_task
            .as_ref()
            .and_then(|task| task.poll_or(|| Err(LOST.to_string())));
        if let Some(result) = enumerated {
            self.auths_task = None;
            self.auth_index = 0;
            match result {
                Ok(auths) => {
                    self.authenticators = auths;
                    if self.action.is_none() {
                        self.status = Status::default();
                    }
                }
                Err(error) => {
                    self.authenticators.clear();
                    self.status = Status::new(Level::Error, format!("Error: {error}"));
                }
            }
        }

        let finished = self.action.as_ref().and_then(|(action, task)| {
            task.poll_or(|| Err(LOST.to_string()))
                .map(|result| (*action, result))
        });
        if let Some((action, result)) = finished {
            self.action = None;
            self.finish_action(action, result);
        }
    }

    fn finish_action(&mut self, action: Action, result: Result<String, String>) {
        match result {
            Ok(text) => {
                if action == Action::Register {
                    self.test_enabled = true;
                }
                self.status = Status::new(Level::Ready, text);
            }
            Err(text) => self.status = Status::new(Level::Error, text),
        }
    }

    fn refresh_authenticators(&mut self, ctx: &Context) {
        let host = self.fields.server_host.trim().to_string();
        if host.is_empty() {
            return;
        }
        let verify_ssl = self.fields.verify_ssl;
        self.status = Status::new(Level::Busy, "Querying authenticators...");
        self.auths_task = Some(Task::spawn(ctx, move || {
            callbacks::enumerate_authenticators(&host, verify_ssl)
        }));
    }

    fn register(&mut self, ctx: &Context) {
        let hostname = self.fields.server_host.trim().to_string();
        let username = self.fields.username.trim().to_string();
        let password = self.fields.password.clone();
        if hostname.is_empty() || username.is_empty() || password.is_empty() {
            self.status = Status::new(Level::Error, "Hostname, username and password are required");
            return;
        }
        let Some(interface) = self.first_interface() else {
            return;
        };

        let input = RegistrationInput {
            hostname,
            verify_ssl: self.fields.verify_ssl,
            auth: self.selected_auth(),
            username,
            password,
            ciphers: self.fields.ssl_ciphers.trim().to_string(),
            log_level: ((self.fields.log_level as u8).min(4)).into(),
            pre_command: self.fields.preconnect_cmd.trim().to_string(),
            runonce_command: self.fields.runonce_cmd.trim().to_string(),
            post_command: self.fields.postconfig_cmd.trim().to_string(),
            interface,
        };
        let operations = self.operations.clone();
        self.status = Status::new(Level::Busy, "Registering...");
        self.action = Some((
            Action::Register,
            Task::spawn(ctx, move || callbacks::register(input, operations.as_ref())),
        ));
    }

    fn first_interface(&self) -> Option<NetworkInterface> {
        match self.operations.get_first_network_interface() {
            Ok(interface) => Some(interface),
            Err(error) => {
                log::error!("No network interfaces found: {}", error);
                let _ = rfd::MessageDialog::new()
                    .set_level(rfd::MessageLevel::Error)
                    .set_title("Network Error")
                    .set_description("No network interfaces found, cannot continue")
                    .show();
                None
            }
        }
    }

    fn test_connection(&mut self, ctx: &Context) {
        match callbacks::registered_config() {
            Err(error) => self.status = Status::new(Level::Error, error),
            Ok(cfg) => {
                self.status = Status::new(Level::Busy, "Testing connection...");
                self.action = Some((
                    Action::Test,
                    Task::spawn(ctx, move || callbacks::test_connection(cfg)),
                ));
            }
        }
    }

    fn general_tab(&mut self, ui: &mut Ui, ctx: &Context) {
        let names = self.auth_names();

        let mut ssl_mode = usize::from(self.fields.verify_ssl);
        widgets::row(ui, "SSL Validation:", |ui, label| {
            index_combo(ui, "ssl-mode", label, &mut ssl_mode, &SSL_MODES)
        });
        self.fields.verify_ssl = ssl_mode == 1;

        let host_response = widgets::row(ui, "UDS Broker Host:", |ui, label| {
            let width = ui.available_width();
            ui.add(
                TextEdit::singleline(&mut self.fields.server_host)
                    .hint_text("broker.example.com")
                    .desired_width(width),
            )
            .labelled_by(label)
        });
        if host_response.lost_focus() {
            self.refresh_authenticators(ctx);
        }

        widgets::row(ui, "Authenticator:", |ui, label| {
            index_combo(ui, "authenticator", label, &mut self.auth_index, &names)
        });

        widgets::row(ui, "Username:", |ui, label| {
            let width = ui.available_width();
            ui.add(TextEdit::singleline(&mut self.fields.username).desired_width(width))
                .labelled_by(label)
        });

        widgets::row(ui, "Password:", |ui, label| {
            let width = ui.available_width();
            ui.add(
                TextEdit::singleline(&mut self.fields.password)
                    .password(true)
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

    fn commands_tab(&mut self, ui: &mut Ui) {
        if path_row(ui, "Preconnect cmd:", &mut self.fields.preconnect_cmd) {
            browse(&mut self.fields.preconnect_cmd);
        }
        if path_row(ui, "Runonce cmd:", &mut self.fields.runonce_cmd) {
            browse(&mut self.fields.runonce_cmd);
        }
        if path_row(ui, "Postconfig cmd:", &mut self.fields.postconfig_cmd) {
            browse(&mut self.fields.postconfig_cmd);
        }
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

    fn tabs(&mut self, ui: &mut Ui, ctx: &Context) {
        ui.horizontal(|ui| {
            for (tab, title) in [
                (Tab::General, "General"),
                (Tab::Commands, "Commands"),
                (Tab::Advanced, "Advanced"),
                (Tab::About, "About"),
            ] {
                ui.selectable_value(&mut self.tab, tab, title);
            }
        });
        ui.separator();
        match self.tab {
            Tab::General => self.general_tab(ui, ctx),
            Tab::Commands => self.commands_tab(ui),
            Tab::Advanced => self.advanced_tab(ui),
            Tab::About => widgets::about(ui, "UDS Actor", shared::consts::VERSION),
        }
    }

    fn buttons(&mut self, ui: &mut Ui, ctx: &Context) {
        let busy = self.busy();
        let mut register = false;
        let mut test = false;
        let mut close = false;
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                register = ui
                    .add_enabled(!busy, widgets::primary_button("Register", REGISTER_WIDTH))
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
        if register {
            self.register(ctx);
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
        self.poll_tasks();
        widgets::header(ui, &self.logo, "UDS Actor");
        self.tabs(ui, &ctx);
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

fn path_row(ui: &mut Ui, label: &str, value: &mut String) -> bool {
    widgets::row(ui, label, |ui, label_id| {
        let width = ui.available_width() - BROWSE_WIDTH - theme::SPACING;
        ui.add(TextEdit::singleline(value).desired_width(width))
            .labelled_by(label_id);
        ui.add(widgets::button("Browse", BROWSE_WIDTH)).clicked()
    })
}

fn browse(value: &mut String) {
    if let Some(path) = rfd::FileDialog::new().pick_file() {
        *value = path.to_string_lossy().into_owned();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};
    use shared::system::new_system;

    fn harness() -> Harness<'static, ConfigApp> {
        let ctx = Context::default();
        let app = ConfigApp::new(&ctx, new_system(), Fields::default(), false);
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
    fn register_without_required_fields_reports_it() {
        let mut harness = harness();
        harness.get_by_label("Register").click();
        harness.run();
        harness.get_by_label("Hostname, username and password are required");
    }
}
