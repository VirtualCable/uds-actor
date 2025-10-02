// gui.rs
use fltk::{app, button::Button, draw, enums::Font, frame::Frame, prelude::*, window::Window};
use std::{
    sync::{Arc, Mutex},
    thread,
};
use tokio::sync::{mpsc, oneshot};

use crate::log;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxResult {
    Ok,
    No,
}

#[derive(Debug, Clone, Copy)]
pub enum MessageBoxButtons {
    Ok,
    YesNo,
}

#[derive(Debug)]
enum GuiCommand {
    Show {
        title: String,
        message: String,
        buttons: MessageBoxButtons,
        respond_to: oneshot::Sender<MessageBoxResult>,
    },
    CloseAll,
    Quit,
}

#[derive(Clone)]
pub struct GuiHandle {
    sender: mpsc::UnboundedSender<GuiCommand>,
}

impl GuiHandle {
    pub fn new() -> Self {
        let (tx, mut rx) = mpsc::unbounded_channel::<GuiCommand>();

        // Dedicated thread for FLTK
        thread::spawn(move || {
            let app = app::App::default();

            // Explicit main loop
            loop {
                app.wait();
                while let Ok(cmd) = rx.try_recv() {
                    log::debug!("GUI: Received command: {:?}", cmd);
                    match cmd {
                        GuiCommand::Show {
                            title,
                            message,
                            buttons,
                            respond_to,
                        } => {
                            log::debug!("GUI: Showing message box: {} - {}", title, message);
                            show_messagebox(&title, &message, buttons, respond_to);
                        }
                        GuiCommand::CloseAll => {
                            log::debug!("GUI: Closing all windows");
                            if let Some(wins) = fltk::app::windows() {
                                for mut w in wins {
                                    let win: Window = unsafe { w.into_widget() };
                                    if w.shown() {
                                        w.hide();
                                        fltk::window::Window::delete(win);
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
            }
        });

        GuiHandle { sender: tx }
    }

    pub async fn message_dialog(
        &self,
        title: &str,
        message: &str,
    ) -> anyhow::Result<MessageBoxResult> {
        log::debug!("GUI: Showing message dialog: {} - {}", title, message);
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(GuiCommand::Show {
                title: title.to_string(),
                message: message.to_string(),
                buttons: MessageBoxButtons::Ok,
                respond_to: tx,
            })
            .map_err(|_| anyhow::anyhow!("GUI thread stopped"))?;
        app::awake();
        let res = rx.await?;
        Ok(res)
    }

    pub async fn yesno_dialog(
        &self,
        title: &str,
        message: &str,
    ) -> anyhow::Result<MessageBoxResult> {
        log::debug!("GUI: Showing yes/no dialog: {} - {}", title, message);
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(GuiCommand::Show {
                title: title.to_string(),
                message: message.to_string(),
                buttons: MessageBoxButtons::YesNo,
                respond_to: tx,
            })
            .map_err(|_| anyhow::anyhow!("GUI thread stopped"))?;
        app::awake();
        let res = rx.await?;
        Ok(res)
    }

    pub fn shutdown(&self) {
        log::debug!("GUI: Shutting down");
        let _ = self.sender.send(GuiCommand::Quit);
        app::awake();
    }

    pub fn close_all_windows(&self) {
        log::debug!("GUI: Closing all windows");
        let _ = self.sender.send(GuiCommand::CloseAll);
        app::awake();
    }
}

impl Default for GuiHandle {
    fn default() -> Self {
        Self::new()
    }
}

fn show_messagebox(
    title: &str,
    message: &str,
    buttons: MessageBoxButtons,
    respond_to: oneshot::Sender<MessageBoxResult>,
) {
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

    let respond_to = Arc::new(Mutex::new(Some(respond_to)));

    match buttons {
        MessageBoxButtons::Ok => {
            let respond_to = respond_to.clone();
            let mut ok = Button::new(width / 2 - 40, height - 50, 80, 30, "Ok");
            ok.set_callback({
                let mut window = window.clone();
                move |_| {
                    if let Some(tx) = respond_to.lock().unwrap().take() {
                        let _ = tx.send(MessageBoxResult::Ok);
                    }
                    window.hide();
                    fltk::window::Window::delete(window.clone());
                }
            });
        }
        MessageBoxButtons::YesNo => {
            let respond_to_yes = respond_to.clone();
            let mut yes = Button::new(width / 2 - 90, height - 50, 80, 30, "Yes");
            yes.set_callback({
                let mut window = window.clone();
                move |_| {
                    if let Some(tx) = respond_to_yes.lock().unwrap().take() {
                        let _ = tx.send(MessageBoxResult::Ok);
                    }
                    window.hide();
                    fltk::window::Window::delete(window.clone());
                }
            });

            let respond_to_no = respond_to.clone();
            let mut no = Button::new(width / 2 + 10, height - 50, 80, 30, "No");
            no.set_callback({
                let mut window = window.clone();
                move |_| {
                    if let Some(tx) = respond_to_no.lock().unwrap().take() {
                        let _ = tx.send(MessageBoxResult::No);
                    }
                    window.hide();
                    fltk::window::Window::delete(window.clone());
                }
            });
        }
    }

    // Handle window close (X) button
    window.handle({
        let respond_to = respond_to.clone();
        move |w, ev| {
            if ev == fltk::enums::Event::Close {
                if let Some(tx) = respond_to.lock().unwrap().take() {
                    let _ = tx.send(MessageBoxResult::No);
                }
                w.hide();
                fltk::window::Window::delete(w.clone());
                true
            } else {
                false
            }
        }
    });

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
        tokio::spawn(async move {
            let res = gui_task
                .yesno_dialog("Confirm", "Do you want to proceed?")
                .await;
            println!("Yes/No dialog result: {:?}", res);
        });
        let gui_task = gui.clone();
        tokio::spawn(async move {
            let res = gui_task
                .message_dialog("Test", "This is a test message with probable several lines and some more text\n even with a newline\n or two")
                .await;
            println!("Message dialog result: {:?}", res);
        });
        // Wait some time to allow interaction
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        gui.close_all_windows();
        gui.shutdown();
    }
}
