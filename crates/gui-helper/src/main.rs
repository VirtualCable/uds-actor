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
#![cfg_attr(not(test), windows_subsystem = "windows")]

use slint::Timer;
use std::rc::Rc;

const SIGNAL_FILE: &str = "uds-actor-gui-close-all";

slint::include_modules!();

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
async fn main() {
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
    let ui = AppWindow::new().unwrap();

    ui.set_title_text(title.into());
    ui.set_message_text(message.into());

    let ui_handle = ui.as_weak();
    ui.on_ok_clicked(move || {
        if let Some(ui) = ui_handle.upgrade() {
            ui.hide().unwrap();
        }
    });

    let timer = Rc::new(Timer::default());
    let ui_handle2 = ui.as_weak();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(500),
        move || {
            let signal_file = std::env::temp_dir().join(SIGNAL_FILE);
            if signal_file.exists() {
                let _ = std::fs::remove_file(&signal_file);
                if let Some(ui) = ui_handle2.upgrade() {
                    ui.hide().unwrap();
                }
            }
        },
    );

    ui.run().unwrap();
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
