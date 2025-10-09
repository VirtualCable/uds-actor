// Copyright (c) 2025 Virtual Cable S.L.U.
// All rights reserved.
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
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
*/
use fltk::{app, button::Button, draw, enums::Font, frame::Frame, prelude::*, window::Window};
use std::thread;

use crate::log;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxResult {
    Ok,
    No,
}

#[derive(Debug)]
enum GuiCommand {
    Show { title: String, message: String },
    CloseAll,
    Quit,
}

#[derive(Clone)]
pub struct GuiHandle {
    sender: fltk::app::Sender<GuiCommand>,
}

impl GuiHandle {
    pub fn new() -> Self {
        let (tx, rx) = fltk::app::channel::<GuiCommand>();

        // Dedicated thread for FLTK
        thread::spawn(move || {
            let app = app::App::default();

            // Explicit main loop
            loop {
                // If no events, wait a bit
                if !app.wait() {
                    std::thread::sleep(std::time::Duration::from_millis(100)); // avoid busy loop
                }
                // Check if we have some command
                while let Some(cmd) = rx.recv() {
                    log::debug!("GUI: Received command: {:?}", cmd);
                    match cmd {
                        GuiCommand::Show { title, message } => {
                            log::debug!("GUI: Showing message box: {} - {}", title, message);
                            show_messagebox(&title, &message);
                        }
                        GuiCommand::CloseAll => {
                            log::debug!("GUI: Closing all windows");
                            if let Some(wins) = fltk::app::windows() {
                                for mut w in wins {
                                    let win: Window = unsafe { w.into_widget() };
                                    if w.shown() {
                                        w.hide();
                                        Window::delete(win);
                                    }
                                }
                            }
                        }
                        GuiCommand::Quit => {
                            log::debug!("GUI: Quitting");
                            return; // exit the GUI thread
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(100)); // avoid busy loop
            }
        });

        GuiHandle { sender: tx }
    }

    pub async fn message_dialog(&self, title: &str, message: &str) {
        log::debug!("GUI: Showing message dialog: {} - {}", title, message);
        self.sender.send(GuiCommand::Show {
            title: title.to_string(),
            message: message.to_string(),
        });
    }

    pub fn shutdown(&self) {
        log::debug!("GUI: Shutting down");
        self.sender.send(GuiCommand::Quit);
    }

    pub fn close_all_windows(&self) {
        log::debug!("GUI: Closing all windows");
        self.sender.send(GuiCommand::CloseAll);
    }
}

impl Default for GuiHandle {
    fn default() -> Self {
        Self::new()
    }
}

fn show_messagebox(title: &str, message: &str) {
    // Split message en líneas
    let lines = split_message(message, 64);

    // Fixe font and size
    let font = Font::Helvetica;
    let font_size = 14;
    draw::set_font(font, font_size);

    // Line height
    let char_height = draw::measure("A", false).1 + 4;

    // maximum width of the lines
    let max_width = lines
        .iter()
        .map(|l| draw::measure(l, false).0)
        .max()
        .unwrap_or(200);

    let width = (max_width + 32).max(240);
    let height = (lines.len() as i32 * char_height) + 100;

    let mut window = Window::new(100, 100, width, height, title).center_screen();

    // Añadir un Frame por cada línea
    for (i, line) in lines.iter().enumerate() {
        let mut frame = Frame::new(
            10,
            10 + (i as i32 * char_height),
            width - 20,
            char_height,
            line.as_str(),
        );
        frame.set_label_size(font_size);
        frame.set_label_font(font);
    }

    _ = Button::new(width / 2 - 40, height - 50, 80, 30, "Ok");

    window.end();
    window.show();
}

fn split_message(msg: &str, max_len: usize) -> Vec<String> {
    let mut lines = Vec::new();

    // First split by explicit newlines
    for paragraph in msg.split('\n') {
        let mut current = paragraph.trim();

        while !current.is_empty() {
            if current.len() <= max_len {
                lines.push(current.to_string());
                break;
            }
            // find last whitespace before max_len
            let split_at = current[..max_len]
                .rfind(char::is_whitespace)
                .unwrap_or(max_len);
            let (line, rest) = current.split_at(split_at);
            lines.push(line.trim().to_string());
            current = rest.trim();
        }

        // If the paragraph was empty, preserve the blank line
        if paragraph.is_empty() {
            lines.push(String::new());
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requires GUI interaction"]
    async fn test_message_dialog() {
        log::setup_logging("debug", log::LogType::Tests);
        let gui = GuiHandle::new();
        let gui_task = gui.clone();
        gui_task
            .message_dialog("Confirm", "First dialog text\nWith a newline")
            .await;
        gui_task
                .message_dialog("Test", "This is a test message with probable several lines and some more text\n even with a newline\n or two")
                .await;
        // Wait some time to allow interaction
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        gui.close_all_windows();
        gui.shutdown();
    }
}
