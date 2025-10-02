use async_trait::async_trait;

// Common actions trait for different platforms
#[async_trait]
pub trait Actions: Send + Sync {
    async fn screenshot(&self) -> anyhow::Result<Vec<u8>>;
    async fn run_script(&self, script: &str) -> anyhow::Result<String>;

    // Default implementation for notifying the user: closes dialogs and shows
    // a notification dialog. Implementations may override if platform-specific
    // behavior is required.
    async fn notify_user(&self, message: &str, gui: crate::gui::GuiHandle) -> anyhow::Result<()> {
        crate::log::info!("Notify user: {}", message);
        let message = message.to_string();
        // ensure_dialogs_closed().await;
        // Execute the dialog on a background thread
        tokio::spawn(async move {
            _ = gui.message_dialog("Notification", &message).await;
        });
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub use crate::windows::actions::new_actions;

#[cfg(unix)]
pub use crate::unix::actions::new_actions;
