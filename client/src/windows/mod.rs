use crate::session::SessionManagement;
use shared::{
    log,
    sync::OnceSignal,
    windows::{MsgWindow, WindowsEvent},
};

#[allow(dead_code)]
pub struct WindowsSessionManager {
    windows_stop_event: WindowsEvent,
    stop: OnceSignal,
}

impl WindowsSessionManager {
    pub async fn new(stop: OnceSignal) -> Self {
        // Create the event to signal the window to stop
        let stop_event = WindowsEvent::new();
        // Launch the window task in a dedicated thread
        let mut msg_window = MsgWindow::new(stop_event.clone());
        std::thread::spawn(move || {
            msg_window.task();
        });

        // Flag to signal task is running
        let flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        // A watchdog to monitor the stop or the msg_window event and set counterpart
        tokio::spawn({
            let flag = flag.clone();
            let stop_event = stop_event.clone();
            let stop = stop.clone();
            async move {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                tokio::select! {
                    _ = stop.wait() => {
                        log::debug!("WindowsSessionManager: Stop signal received, signaling windows event");
                        stop_event.signal();
                    }
                    _ = stop_event.wait_async() => {
                        log::debug!("WindowsSessionManager: Windows event signaled, setting stop signal");
                        stop.set();
                    }
                }
            }
        });

        // Wait until the watchdog is running
        while !flag.load(std::sync::atomic::Ordering::SeqCst) {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }

        Self {
            windows_stop_event: stop_event,
            stop,
        }
    }
}

#[async_trait::async_trait]
impl SessionManagement for WindowsSessionManager {
    fn get_stop(&self) -> OnceSignal {
        self.stop.clone()
    }
    async fn is_running(&self) -> bool {
        !self.windows_stop_event.is_set()
    }
    async fn stop(&self) {
        self.windows_stop_event.signal();
        log::debug!("Windows session close event signaled");
    }
}

pub async fn new_session_manager(stop: OnceSignal) -> std::sync::Arc<dyn SessionManagement + Send + Sync> {
    std::sync::Arc::new(WindowsSessionManager::new(stop).await)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_windows_session_close_by_event() {
        shared::log::setup_logging("debug", shared::log::LogType::Tests);
        let session_close = WindowsSessionManager::new(OnceSignal::new()).await;
        let event = session_close.windows_stop_event.clone();
        let stop = session_close.get_stop();
        let _fake_closer = tokio::spawn({
            async move {
            session_close.get_stop().wait().await;
        }});
        // wait a bit to simulate work
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        event.signal();
        // Wait a bit to ensure the event has been handled
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        // stop should be set now
        assert!(stop.is_set());
    }

    #[tokio::test]
    async fn test_windows_session_close_by_stop() {
        shared::log::setup_logging("debug", shared::log::LogType::Tests);
        let session_close = WindowsSessionManager::new(OnceSignal::new()).await;
        let event = session_close.windows_stop_event.clone();
        let stop = session_close.get_stop();
        let _fake_closer = tokio::spawn({
            async move {
            session_close.get_stop().wait().await;
        }});
        // wait a bit to simulate work
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        stop.set();
        // Wait a bit to ensure the event has been handled
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        // event should be set now
        assert!(event.is_set());
    }
}
