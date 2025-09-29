use async_trait::async_trait;
use crate::gui::{ensure_dialogs_closed, message_dialog};

// Common actions trait for different platforms
#[async_trait]
pub trait Actions: Send + Sync {
    async fn logoff(&self) -> anyhow::Result<()>;
    async fn screenshot(&self) -> anyhow::Result<Vec<u8>>;
    async fn run_script(&self, script: &str) -> anyhow::Result<String>;
    async fn show_message(&self, message: &str) -> anyhow::Result<String>;

    // Default implementation for notifying the user: closes dialogs and shows
    // a notification dialog. Implementations may override if platform-specific
    // behavior is required.
    async fn notify_user(&self, message: &str) -> anyhow::Result<()> {
        crate::log::info!("Notify user: {}", message);
        ensure_dialogs_closed().await;
        message_dialog("Notification", message).await?;
        Ok(())
    }
}


#[cfg(target_os = "windows")]
pub use crate::windows::actions::new_actions;

#[cfg(unix)]
pub use crate::unix::actions::new_actions;