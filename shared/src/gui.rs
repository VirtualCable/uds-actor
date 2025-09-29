use fltk::{app, button::Button, draw, enums::Font, frame::Frame, prelude::*, window::Window};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

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

/// Blocking generic messagebox
fn messagebox(
    title: &str,
    message: &str,
    buttons: MessageBoxButtons,
    max_chars: usize,
) -> anyhow::Result<MessageBoxResult> {
    // Check if another messagebox is already active
    if MESSAGEBOX_ACTIVE
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err(anyhow::Error::msg("Another messagebox is already active"));
    }

    let app = app::App::default();
    app::add_idle3(move |_| {
        if SHUTDOWN.load(Ordering::SeqCst) {
            app.quit();
        }
    });

    // Split message into lines
    let lines = split_message(message, max_chars);

    // Fixed font and size
    let font = Font::Helvetica;
    let font_size = 14;
    // Ensure the font is set
    draw::set_font(font, font_size);
    // Measure character height with some padding
    let char_height = draw::measure("A", false).1 + 4;

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

    crate::log::debug!(
        "MessageBox first line: '{}' = {:?}",
        lines.first().unwrap_or(&"".to_string()),
        draw::measure(lines.first().unwrap_or(&"".to_string()).as_str(), false)
    );

    let mut wind = Window::new(100, 100, width, height, title).center_screen();

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

    wind.end();
    wind.show();

    app.run().unwrap();

    // Reset the guard when finished
    MESSAGEBOX_ACTIVE.store(false, Ordering::SeqCst);

    Ok(*result.lock().unwrap())
}

/// Async helper: muestra un diálogo con solo Ok, no devuelve nada
pub async fn message_dialog(title: &str, message: &str) -> anyhow::Result<MessageBoxResult> {
    let title = title.to_string();
    let message = message.to_string();
    tokio::task::spawn_blocking(move || {
        messagebox(&title, &message, MessageBoxButtons::Ok, 64).map_err(anyhow::Error::msg)
    })
    .await
    .unwrap_or_else(|_| Err(anyhow::Error::msg("Task Join Error")))
}

/// Async helper: muestra un diálogo Yes/No y devuelve el resultado
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
    // Signal shutdown
    SHUTDOWN.store(true, Ordering::SeqCst);

    // Poll MESSAGEBOX_ACTIVE up to 10 times, 100ms each
    // Should be enough to close any active messagebox
    let mut closed = false;
    for _ in 0..10 {
        if !MESSAGEBOX_ACTIVE.load(Ordering::SeqCst) {
            closed = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    if !closed {
        crate::log::warn!("ensure_window_slot: Timeout waiting for messagebox to close");
    }

    // Reset shutdown flag so new dialogs can open
    SHUTDOWN.store(false, Ordering::SeqCst);
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
    use super::split_message;
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
}
