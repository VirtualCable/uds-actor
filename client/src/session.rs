use crate::rest::api::ClientRestApi;

#[async_trait::async_trait]
#[allow(dead_code)]
pub trait SessionManagement {
    async fn wait(&self);
    async fn is_running(&self) -> bool;
    async fn stop(&self);

    // We are not going to build so many times a ClientRestApi, so this is fine
    fn get_api(&self) -> ClientRestApi {
        ClientRestApi::new("https://127.0.0.1:43910", false)
    }
}

#[cfg(windows)]
pub use crate::windows::new_session_manager;

// Linux and macOS implementation are identical
#[cfg(unix)]
pub use crate::linux::UnixSessionManager as SessionClose;
#[cfg(unix)]
pub use crate::linux::new_session_manager;
