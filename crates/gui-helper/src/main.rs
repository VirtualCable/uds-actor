// Copyright (c) 2025 Virtual Cable S.L.U.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
//
//    * Redistributions of source code must retain the above copyright notice,
//      this list of conditions and the following disclaimer.
//    * Redistributions in binary form must reproduce the above copyright notice,
//      this list of conditions and the following disclaimer in the documentation
//      and/or other materials provided with the distribution.
//    * Neither the name of Virtual Cable S.L.U. nor the names of its contributors
//      may be used to endorse or promote products derived from this software
//      without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
/*!
Author: Adolfo Gómez, dkmaster at dkmon dot com
Author: Janier Rodríguez, jrodriguez at virtualcable dot es
Author: Andres Schumann, aschumann at virtualcable dot es
*/
#![cfg_attr(not(test), windows_subsystem = "windows")]

use eframe::egui::{self, Ui, ViewportCommand};
use gui_kit::{logo, theme, widgets};
use std::time::Duration;

const SIGNAL_FILE: &str = "uds-actor-gui-close-all";
const SIGNAL_POLL: Duration = Duration::from_millis(500);

/*
This binary exists to solve a very specific problem:

FLTK (and Xlib) will call `exit(1)` if the X server dies unexpectedly.
That means: if you're running a GUI inside your main process and the X server closes/crashes,
your entire app dies — no cleanup, no mercy.

We need to keep the main app alive to log the event and clean up properly.

So instead, we isolate the GUI in a separate process — this one.
It shows message dialogs, and nothing else.
The main app stays alive, logs the event, and can clean up properly.

Communication is minimal:
- To show a message, the main app launches this binary with arguments.
- To request all windows to close, it creates a temp file named `uds-actor-gui-close-all`.
- This binary checks for that file periodically and exits if found.
*/
#[tokio::main]
async fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        eprintln!("Usage: gui-helper [message-dialog] <title> <message>");
        std::process::exit(1);
    }

    let command = &args[1];
    if command != "message-dialog" {
        eprintln!("Unknown command: {}", command);
        std::process::exit(1);
    }

    show_messagebox(args[2].clone(), args[3].clone())
}

fn show_messagebox(title: String, message: String) -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title.clone())
            .with_inner_size([420.0, 200.0])
            .with_min_inner_size([400.0, 150.0])
            .with_icon(logo::window_icon()),
        ..Default::default()
    };
    eframe::run_native(
        &title,
        options,
        Box::new(move |cc| {
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(MessageApp::new(message)))
        }),
    )
}

struct MessageApp {
    message: String,
    show_about: bool,
}

impl MessageApp {
    fn new(message: String) -> Self {
        Self {
            message,
            show_about: false,
        }
    }

    fn body(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new(&self.message).size(14.0));
        });

        ui.vertical_centered(|ui| {
            if ui.add(widgets::primary_button("OK", 100.0)).clicked() {
                ctx.send_viewport_cmd(ViewportCommand::Close);
            }
            let about_link = ui.link(egui::RichText::new("About").size(11.0).color(theme::ACCENT));
            if about_link.clicked() {
                self.show_about = !self.show_about;
            }
            if self.show_about {
                widgets::about(ui, "UDS Actor Client", shared::consts::VERSION);
            }
        });
    }

    fn signal_requested_close(&self) -> bool {
        let signal_file = std::env::temp_dir().join(SIGNAL_FILE);
        if signal_file.exists() {
            let _ = std::fs::remove_file(&signal_file);
            return true;
        }
        false
    }
}

impl eframe::App for MessageApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        theme::apply(ui.ctx());
        ui.ctx().request_repaint_after(SIGNAL_POLL);
        if self.signal_requested_close() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::central_panel(ui.style()).inner_margin(theme::panel_margin()))
            .show(ui, |ui| self.body(ui));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};

    fn harness() -> Harness<'static, MessageApp> {
        let app = MessageApp::new("Test message".to_string());
        Harness::new_ui_state(|ui, app: &mut MessageApp| app.body(ui), app)
    }

    #[test]
    fn about_link_toggles_the_product_panel() {
        let mut harness = harness();
        assert!(
            harness
                .query_by_label("Copyright (c) Virtual Cable S.L.U.")
                .is_none()
        );

        harness.get_by_label("About").click();
        harness.run();
        harness.get_by_label("Copyright (c) Virtual Cable S.L.U.");

        harness.get_by_label("About").click();
        harness.run();
        assert!(
            harness
                .query_by_label("Copyright (c) Virtual Cable S.L.U.")
                .is_none()
        );
    }

    #[test]
    #[ignore = "Requires GUI interaction"]
    fn test_message_dialog() {
        let title = "Test Title";
        let message = "This is a test message to verify that the message dialog works correctly.\n\
                       It should handle multiple lines and proper word wrapping.";
        let _ = show_messagebox(title.to_string(), message.to_string());
    }
}
