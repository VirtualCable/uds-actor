use anyhow::Result;
use tokio::process::Command;

use shared::log;


// We have created a separate gui helper because on linux
// at session close the X windows (xrdp for example) destroys de X server.
// fltk fails and do a Fl::fatal, that in turn executes "exit(1)" which makes
// the whole process to exit with error, and that is not desired.
// This way, we can control our life cycle better.

const GUI_HELPER_EXE: &str = if cfg!(windows) {
    "gui-helper.exe"
} else {
    "gui-helper"
};
const SIGNAL_FILE: &str = "uds-actor-gui-close-all";

pub async fn message_dialog(title: &str, message: &str) -> Result<()> {
    let title = title.to_string();
    let message = message.to_string();
    tokio::spawn(async move {
        exec_message_dialog(&title, &message).await.ok();
    });
    Ok(())
}

async fn exec_message_dialog(title: &str, message: &str) -> Result<()> {
    log::debug!("Showing message dialog: {} - {}", title, message);
    let signal_file = std::env::temp_dir().join(SIGNAL_FILE);
    log::debug!("Using signal file: {:?}", signal_file);
    let _ = std::fs::remove_file(&signal_file); // Remove any existing signal file

    let gui_path = std::env::current_exe()?
        .parent()
        .unwrap()
        .join(GUI_HELPER_EXE)
        .canonicalize()?;

    log::debug!("Using gui path: {:?}", gui_path);

    let status = Command::new(gui_path)
        .arg("message-dialog")
        .arg(title)
        .arg(message)
        .status()
        .await?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(
            "Failed to show message dialog, exit code: {:?}",
            status.code()
        ))
    }
}

// close all windows, will create a temporary filename on TempDir
// named uds-actr-gui-close-all to signal the gui-helper to close all windows
pub async fn close_all_windows() -> Result<()> {
    log::debug!("Closing all windows");
    let signal_file = std::env::temp_dir().join(SIGNAL_FILE);
    std::fs::File::create(&signal_file)?;

    Ok(())
}
