use shared::{log, sync::OnceSignal};

use crate::session::SessionManagement;

pub struct UnixSessionManager {
    stop: OnceSignal,
}

impl UnixSessionManager {
    pub async fn new(stop: OnceSignal) -> Self {
        log::debug!("************* Creating UnixSessionManager ***********");
        Self { stop }
    }
}

#[async_trait::async_trait]
impl SessionManagement for UnixSessionManager {
    fn get_stop(&self) -> OnceSignal {
        self.stop.clone()
    }

    async fn is_running(&self) -> bool {
        !self.stop.is_set()
    }

    async fn stop(&self) {
        self.stop.set();
        log::debug!("Unix session close event signaled");
    }
}

pub async fn new_session_manager(stop: OnceSignal) -> std::sync::Arc<dyn SessionManagement + Send + Sync> {
    std::sync::Arc::new(UnixSessionManager::new(stop).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_unix_session_close() {
        let stop = OnceSignal::new();
        let session_close = UnixSessionManager::new(stop.clone()).await;
        let _fake_closer = tokio::spawn(async move {
            session_close.get_stop().wait().await;
        });
        // Wait a bit to simulate waiting
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        stop.set();
        // Wait a bit to ensure the event is handled
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
