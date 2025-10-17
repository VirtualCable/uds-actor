use anyhow::Result;
use tokio::signal::unix::{SignalKind, signal};

use shared::{
    log,
    sync::OnceSignal,
};

use crate::session::SessionManagement;

pub struct UnixSessionManager {
    stop_event: OnceSignal,
}

impl UnixSessionManager {
    pub fn new() -> Self {
        log::debug!("************* Creating UnixSessionManager ***********");
        Self {
            stop_event: OnceSignal::new(),
        }
    }
}

#[async_trait::async_trait]
impl SessionManagement for UnixSessionManager {
    async fn wait(&self) {
        // Listen for SIGTERM or SIGINT
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = sigterm.recv() => {
                log::debug!("Received SIGTERM");
                self.stop_event.set();
            },
            _ = sigint.recv() => {
                log::debug!("Received SIGINT");
                self.stop_event.set();
            },
            _ = self.stop_event.wait() => {
                log::debug!("Unix session close event received");
            }
        }
    }

    async fn is_running(&self) -> bool {
        !self.stop_event.is_set()
    }

    async fn stop(&self) {
        self.stop_event.set();
        log::debug!("Unix session close event signaled");
    }

    async fn wait_timeout(&self, timeout: std::time::Duration) -> Result<()> {
        self.stop_event.wait_timeout(timeout).await
    }
}

pub fn new_session_manager() -> std::sync::Arc<dyn SessionManagement + Send + Sync> {
    std::sync::Arc::new(UnixSessionManager::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_unix_session_close() {
        let session_close = UnixSessionManager::new();
        let event = session_close.stop_event.clone();
        let _fake_closer = tokio::spawn(async move {
            session_close.wait().await;
        });
        // Wait a bit to simulate waiting
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        event.set();
        // Wait a bit to ensure the event is handled
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
