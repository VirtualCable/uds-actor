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

const SIGNAL_FILE: &str = "uds-actor-gui-close-all";

// We have created a separate gui helper because on linux
// at session close the X windows (xrdp for example) destroys de X server.
// fltk fails and do a Fl::fatal, that in turn executes "exit(1)" which makes
// the whole process to exit with error, and that is not desired.
// If this fails, only the message dialog will be lost :).

fn main() {
    // Get title and message from args
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 {
        // program name, command, title, message
        eprintln!("Usage: gui-helper [message-dialog] <title> <message>");
        std::process::exit(1);
    }

    let command = &args[1];
    if command != "message-dialog" {
        eprintln!("Unknown command: {}", command);
        std::process::exit(1);
    }
    let title = args[2].clone();
    let message = args[3].clone();

    show_messagebox(&title, &message);
}

fn show_messagebox(title: &str, message: &str) {
    let app = app::App::default();

    // Split message
    let lines = split_message(&message, 64);

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

    // Add a frame for each line
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

    let mut btn = Button::new(width / 2 - 40, height - 50, 80, 30, "Ok");

    // Register button callback to close window
    btn.set_callback({
        let mut win = window.clone();
        move |_| {
            win.hide();
        }
    });

    // Register a timeout callback to check for signal file
    app::add_timeout3(0.5, {
        let window = window.clone();
        move |_| {
            check_signal_file(window.clone());
        }
    });

    window.end();
    window.show();

    app.run().unwrap();
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

fn check_signal_file(mut win: Window) {
    let signal_file = std::env::temp_dir().join(SIGNAL_FILE);
    if signal_file.exists() {
        let _ = std::fs::remove_file(&signal_file);
        win.hide();
    } else {
        app::add_timeout3(0.5, {
            let win = win.clone();
            move |_| {
                check_signal_file(win.clone());
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Requires GUI interaction"]
    fn test_message_dialog() {
        // Setup arguments and environment
        let title = "Test Title";
        let message = "This is a test message to verify that the message dialog works correctly.\n\
                       It should handle multiple lines and proper word wrapping.";
        show_messagebox(title, message);
    }
}
