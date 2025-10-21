use shared::sync::OnceSignal;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait SessionManagement: Send + Sync {
    fn get_stop(&self) -> OnceSignal;
    async fn is_running(&self) -> bool;
    async fn stop(&self);
}

#[cfg(windows)]
pub use crate::windows::new_session_manager;

// Linux and macOS implementation are identical
#[cfg(unix)]
pub use crate::unix::new_session_manager;
