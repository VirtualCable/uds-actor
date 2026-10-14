use shared::{log, sync::OnceSignal};

use crate::session::SessionManagement;

pub struct UnixSessionManager {
    stop: OnceSignal,
}

impl UnixSessionManager {
    pub async fn new(stop: OnceSignal) -> Self {
        log::debug!("************* Creating UnixSessionManager ***********");

        // Session-end detection. Note: nothing here may touch X — the X
        // server can die abruptly at session close and any X connection
        // would kill us before cleanup (that's why the GUI lives in the
        // gui-helper process). Signals and logind do not depend on X.
        spawn_signal_handlers(stop.clone());

        // On Linux, also watch logind: when our session is removed
        // (xrdp session closed, user slice stopped, ...) we get
        // SessionRemoved via D-Bus and can notify the logout cleanly.
        #[cfg(target_os = "linux")]
        {
            let stop = stop.clone();
            tokio::spawn(async move {
                if let Err(e) =
                    shared::unix::linux::watcher::start_session_watch_task(stop).await
                {
                    log::warn!("Could not start logind session watcher: {}", e);
                }
            });
        }

        Self { stop }
    }
}

// SIGTERM/SIGINT/SIGHUP: sent when the session leader dies, the user slice
// is stopped, or the process is terminated manually. Each one sets stop so
// the main loop performs the broker logout before exiting.
fn spawn_signal_handlers(stop: OnceSignal) {
    use tokio::signal::unix::{SignalKind, signal};

    for (kind, name) in [
        (SignalKind::terminate(), "SIGTERM"),
        (SignalKind::interrupt(), "SIGINT"),
        (SignalKind::hangup(), "SIGHUP"),
    ] {
        match signal(kind) {
            Ok(mut sig) => {
                let stop = stop.clone();
                tokio::spawn(async move {
                    sig.recv().await;
                    log::info!("Received {}, notifying session stop", name);
                    stop.set();
                });
            }
            Err(e) => log::warn!("Could not install {} handler: {}", name, e),
        }
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

pub async fn new_session_manager(
    stop: OnceSignal,
) -> std::sync::Arc<dyn SessionManagement + Send + Sync> {
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
