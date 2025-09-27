#[async_trait::async_trait]

#[allow(dead_code)]
pub trait SessionManagement {
    async fn wait(&self);
    async fn is_running(&self) -> bool;
    async fn stop(&self);
}

#[cfg(windows)]
pub use crate::windows::new_session_manager;

// Linux and macOS implementation are identical
#[cfg(unix)]
pub use crate::linux::UnixSessionManager as SessionClose;
#[cfg(unix)]
pub use crate::linux::new_session_manager;
