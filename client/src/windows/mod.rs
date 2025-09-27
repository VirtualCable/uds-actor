use crate::session::SessionManagement;
use shared::{
    log,
    sync::event::{Event, EventLike},
    windows::MsgWindow,
};

pub struct WindowsSessionManager {
    stop_event: Event,
}

impl WindowsSessionManager {
    pub fn new() -> Self {
        // Create the event to signal the window to stop
        let stop_event = Event::new();
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
}

pub fn new_session_manager() -> impl SessionManagement {
    WindowsSessionManager::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_windows_session_close() {
        let session_close = WindowsSessionManager::new();
        let event = session_close.stop_event.clone();
        let _fake_closer =
        tokio::spawn(async move {
            session_close.wait().await;
        });
        // Esperamos un poco para simular la espera
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        event.signal();
        // Esperamos un poco para asegurarnos de que el evento se ha manejado
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        
    }
}
