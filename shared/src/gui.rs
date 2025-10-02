use fltk::{app, button::Button, draw, enums::Font, frame::Frame, prelude::*, window::Window};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tracing_log::log;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageBoxResult {
    Ok,
    No,
}

impl std::fmt::Display for MessageBoxResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MessageBoxResult::Ok => write!(f, "Ok"),
            MessageBoxResult::No => write!(f, "No"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum MessageBoxButtons {
    Ok,
    YesNo,
}

// Global guard to prevent multiple messageboxes at the same time
static MESSAGEBOX_ACTIVE: AtomicBool = AtomicBool::new(false);
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

static APP: std::sync::OnceLock<fltk::app::App> = std::sync::OnceLock::new();

/// Blocking generic messagebox
fn messagebox(
    title: &str,
    message: &str,
    buttons: MessageBoxButtons,
    max_chars: usize,
) -> anyhow::Result<MessageBoxResult> {
    let app = APP.get_or_init(app::App::default);
    log::debug!("MessageBox app created/initialized");

    log::debug!(
        "Preparing messagebox: {} - {} ({})",
        title,
        message,
        MESSAGEBOX_ACTIVE.load(Ordering::SeqCst)
    );
    // Check if another messagebox is already active
    if MESSAGEBOX_ACTIVE
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        log::warn!("Another messagebox is already active, cannot open a new one");
        return Err(anyhow::Error::msg("Another messagebox is already active"));
    }

    // Split message into lines
    let lines = split_message(message, max_chars);

    // Fixed font and size
    let font = Font::Helvetica;
    let font_size = 14;
    // Ensure the font is set
    draw::set_font(font, font_size);
    // Measure character height with some padding
    let char_height = draw::measure("A", false).1 + 4;
    log::debug!("Character height: {}", char_height);

    // Approximate width using measure
    let max_width = lines
        .iter()
        .map(|l| {
            let (w, _) = draw::measure(l, false);
            w
        })
        .max()
        .unwrap_or(200);

    let width = (max_width + 32).max(240); // give some padding, minimum 240
    let height = (lines.len() as i32 * char_height) + 100;
    log::debug!("MessageBox size: {}x{}", width, height);

    let mut window = Window::new(100, 100, width, height, title).center_screen();

    // Add frames for each line
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

    let result = Arc::new(Mutex::new(MessageBoxResult::No));
    let cb_result = result.clone();

    match buttons {
        MessageBoxButtons::Ok => {
            let mut ok = Button::new(width / 2 - 40, height - 50, 80, 30, "Ok");
            ok.set_label_size(font_size);
            ok.set_label_font(font);
            ok.set_callback({
                move |_| {
                    *cb_result.lock().unwrap() = MessageBoxResult::Ok;
                    app.quit();
                }
            });
        }
        MessageBoxButtons::YesNo => {
            let mut yes = Button::new(width / 2 - 90, height - 50, 80, 30, "Yes");
            let mut no = Button::new(width / 2 + 10, height - 50, 80, 30, "No");

            for b in [&mut yes, &mut no] {
                b.set_label_size(font_size);
                b.set_label_font(font);
            }

            yes.set_callback({
                let cb_result = cb_result.clone();
                move |_| {
                    *cb_result.lock().unwrap() = MessageBoxResult::Ok;
                    app.quit();
                }
            });

            no.set_callback({
                move |_| {
                    *cb_result.lock().unwrap() = MessageBoxResult::No;
                    app.quit();
                }
            });
        }
    }

    log::debug!("MessageBox ready, ending window");
    window.end();
    log::debug!("MessageBox ready, showing");
    window.show();

    // Timeout callback
    let cb_app = app;
    let cb_timeout = move |_handle| {
        if SHUTDOWN.load(Ordering::SeqCst) {
            log::debug!("MessageBox received shutdown signal, closing");
            // Reset SHUTDOWN flag
            SHUTDOWN.store(false, Ordering::SeqCst);
            // Close the app
            cb_app.quit();
        } else {
            // Repeat the timeout
            app::repeat_timeout3(1.0, _handle);
        }
    };

    app::add_timeout3(1.0, cb_timeout);

    log::debug!("MessageBox ready, running app");
    if let Err(e) = app.run() {
        // Reset the guard on error
        log::error!("FLTK run error: {}", e);
        MESSAGEBOX_ACTIVE.store(false, Ordering::SeqCst);
        return Err(anyhow::Error::msg(format!("FLTK run error: {}", e)));
    }

    fltk::window::Window::delete(window);

    // Reset the guard when finished
    MESSAGEBOX_ACTIVE.store(false, Ordering::SeqCst);
    log::debug!("MessageBox closed & marked as not active");

    Ok(*result.lock().unwrap())
}

/// Async helper: shows a dialog with only Ok, does not return anything
pub async fn message_dialog(title: &str, message: &str) -> anyhow::Result<MessageBoxResult> {
    log::debug!("Displaying message dialog: {} - {}", title, message);
    let title = title.to_string();
    let message = message.to_string();
    tokio::task::spawn_blocking(move || {
        messagebox(&title, &message, MessageBoxButtons::Ok, 64).map_err(anyhow::Error::msg)
    })
    .await
    .unwrap_or_else(|_| Err(anyhow::Error::msg("Task Join Error")))
}

/// Async helper: shows a Yes/No dialog and returns the result
pub async fn yesno_dialog(title: &str, message: &str) -> anyhow::Result<MessageBoxResult> {
    let title = title.to_string();
    let message = message.to_string();
    tokio::task::spawn_blocking(move || {
        messagebox(&title, &message, MessageBoxButtons::YesNo, 64).map_err(anyhow::Error::msg)
    })
    .await
    .unwrap_or_else(|_| Err(anyhow::Error::msg("Task Join Error")))
}

/// Async closer: signal shutdown and wait until any active messagebox is gone
pub async fn ensure_dialogs_closed() {
    // If already shutting down, return
    if SHUTDOWN.load(Ordering::SeqCst) {
        log::debug!("ensure_dialogs_closed: Already shutting down, returning");
        return;
    }

    // Signal shutdown
    SHUTDOWN.store(true, Ordering::SeqCst);
    log::debug!("ensure_dialogs_closed: Signaled shutdown, waiting for messagebox to close");

    // Poll MESSAGEBOX_ACTIVE up to 10 times, 100ms each
    // Should be enough to close any active messagebox
    let mut closed = false;
    for _ in 0..10 {
        if !MESSAGEBOX_ACTIVE.load(Ordering::SeqCst) {
            log::debug!("ensure_dialogs_closed: Messagebox closed");
            closed = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
    if !closed {
        log::warn!("ensure_dialogs_closed: Timeout waiting for messagebox to close");
    }

    // Ensure shutdown flag is reset so new dialogs can open
    SHUTDOWN.store(false, Ordering::SeqCst);
    log::debug!("ensure_dialogs_closed: Reset shutdown flag");
}

pub async fn shutdown() {
    SHUTDOWN.store(true, Ordering::SeqCst);
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
    use crate::log;

    #[test]
    fn test_split_message() {
        log::setup_logging("debug", log::LogType::Tests);
        let msg = "This is a test message that should be split into multiple lines based on the maximum length specified.";
        let lines = split_message(msg, 20);
        for line in &lines {
            log::info!("Line: '{}'", line);
            assert!(line.len() <= 20);
        }
        assert_eq!(lines.len(), 7); // 7 lines because of word boundaries
    }

    // Note: The gui test is omitted because it requires a GUI environment to run
    // And will block the test suite
    #[test]
    #[ignore]
    fn test_messagebox() {
        log::setup_logging("debug", log::LogType::Tests);
        let res = messagebox(
            "Test Title",
            "This is a test message to display in the message box. It should handle long messages properly.",
            MessageBoxButtons::YesNo,
            40,
        );
        assert!(res.is_ok());
        log::info!("MessageBox result: {}", res.unwrap());
    }

    // Test that multiple messagedialog works
    #[tokio::test]
    #[ignore] // Ignored because it requires GUI environment and user interaction
    async fn test_multiple_message_dialogs() {
        log::setup_logging("debug", log::LogType::Tests);
        for i in 0..5 {
            let title = format!("Test Dialog {}", i);
            let message = format!("This is test dialog number {}.", i);
            tokio::spawn(async move {
                ensure_dialogs_closed().await;
                let res = message_dialog(&title, &message).await;
                log::info!("Dialog {} result: {:?}", i, res);
                res
            });
            // Wait a bit before starting the next dialog
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
        }
        // Wait a bit to let dialogs finish
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        ensure_dialogs_closed().await;
    }

    async fn notify_user(message: &str) -> anyhow::Result<()> {
        crate::log::info!("Notify user: {}", message);
        let message = message.to_string();
        ensure_dialogs_closed().await;
        // Execute the dialog on a background thread
        tokio::spawn(async move {
            _ = message_dialog("Notification", &message).await;
        });
        Ok(())
    }

    // Test idle dialog closing automatically
    #[tokio::test]
    async fn test_idle_dialog_closing() {
        log::setup_logging("debug", log::LogType::Tests);
        notify_user("This is a test notification that should close automatically in 1 seconds.")
            .await
            .unwrap();
        // Wait 1 second, to repeat the notification
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        ensure_dialogs_closed().await;
        // Wait a bit to see the dialog closed
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        notify_user("This is a test notification that should close automatically in 1 seconds.")
            .await
            .unwrap();
        // Wait 3 seconds to ensure the first dialog is closed
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        // Now ensure all dialogs are closed
        ensure_dialogs_closed().await;
        // Wait a bit to see the dialog closed
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        // Final notification
        notify_user("Final notification after ensuring dialogs are closed.")
            .await
            .unwrap();
        // Wait a bit to see the final dialog
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        ensure_dialogs_closed().await;
    }
}
