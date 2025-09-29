#[async_trait::async_trait]
#[allow(dead_code)]
pub trait SessionManagement: Send + Sync {
    async fn wait(&self);
    /// Returns true if the event was signaled, false if the timeout expired
    async fn wait_timeout(&self, timeout: std::time::Duration) -> bool;
    async fn is_running(&self) -> bool;
    async fn stop(&self);
}

#[cfg(windows)]
pub use crate::windows::new_session_manager;

// Linux and macOS implementation are identical
#[cfg(unix)]
pub use crate::unix::new_session_manager;
