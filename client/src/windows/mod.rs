use anyhow::Result;

use crate::session::SessionManagement;
use shared::{
    log,
    windows::{MsgWindow, WindowsEvent},
};

#[allow(dead_code)]
pub struct WindowsSessionManager {
    stop_event: WindowsEvent,
}

impl WindowsSessionManager {
    pub fn new() -> Self {
        // Create the event to signal the window to stop
        let stop_event = WindowsEvent::new();
        // Launch the window task in a dedicated thread
        let mut msg_window = MsgWindow::new(stop_event.clone());
        std::thread::spawn(move || {
            msg_window.task();
        });

        Self { stop_event }
    }
}

#[async_trait::async_trait]
impl SessionManagement for WindowsSessionManager {
    async fn wait(&self) {
        self.stop_event.wait_async().await;
        log::debug!("Windows session close event received");
    }
    async fn is_running(&self) -> bool {
        !self.stop_event.is_set()
    }
    async fn stop(&self) {
        self.stop_event.signal();
        log::debug!("Windows session close event signaled");
    }
    async fn wait_timeout(&self, timeout: std::time::Duration) -> Result<()> {
        let ev = self.stop_event.clone();
        ev.wait_timeout_async(timeout).await
    }
}

pub async fn new_session_manager() -> std::sync::Arc<dyn SessionManagement + Send + Sync> {
    std::sync::Arc::new(WindowsSessionManager::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_windows_session_close() {
        let session_close = WindowsSessionManager::new();
        let event = session_close.stop_event.clone();
        let _fake_closer = tokio::spawn(async move {
            session_close.wait().await;
        });
        // wait a bit to simulate work
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        event.signal();
        // Wait a bit to ensure the event has been handled
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
