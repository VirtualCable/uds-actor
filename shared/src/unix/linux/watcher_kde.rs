use anyhow::Result;
use futures_util::StreamExt;
use zbus::{Connection, proxy};

use crate::{log, sync::OnceSignal};

#[proxy(
    interface = "org.kde.KSMServerInterface",
    default_path = "/KSMServer",
    default_service = "org.kde.ksmserver"
)]
trait KSMServer {
    // We only care about signals here
    #[zbus(signal)]
    fn logoutRequested(&self, shutdown: i32, confirm: i32, sdtype: i32);
    #[zbus(signal)]
    fn logout(&self, shutdown: i32, confirm: i32, sdtype: i32);
}

/// Check if KDE KSMServer is available on the session bus.
/// Returns true if the service is present, false otherwise.
pub async fn is_kde_session_watcher_available() -> Result<bool> {
    let conn = Connection::session().await?;

    let msg = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "NameHasOwner",
            &("org.kde.ksmserver",),
        )
        .await?;

    let reply: bool = msg.body().deserialize()?;
    Ok(reply)
}

/// Start a background task that watches KDE session signals.
/// It will stop either when `stop` is set or when a session end signal arrives.
async fn kde_session_watcher_task(stop: OnceSignal) -> Result<()> {
    let conn = Connection::session().await?;
    let proxy = KSMServerProxy::new(&conn).await?;

    let mut logout_requested = proxy.receive_logoutRequested().await?;
    let mut logout = proxy.receive_logout().await?;

    log::debug!("KDE session watch task started");
    loop {
        tokio::select! {
            _ = stop.wait() => {
                log::debug!("Stop signal received, ending KDE session watch");
                break;
            }
            msg = logout_requested.next() => {
                if msg.is_some() {
                    log::info!("logoutRequested signal received, preparing to shutdown");
                    stop.set();
                    break;
                }
            }
            msg = logout.next() => {
                if msg.is_some() {
                    log::warn!("logout signal received, session is ending");
                    stop.set();
                    break;
                }
            }
        }
    }
    log::debug!("KDE session watch task finished");
    Ok(())
}

pub async fn create_kde_session_watcher_task(stop: OnceSignal) -> Result<()> {
    if !is_kde_session_watcher_available().await? {
        log::warn!("KDE KSMServer service not available, cannot start session watcher");
        return Err(anyhow::anyhow!(
            "KDE KSMServer service not available"
        ));
    }

    tokio::spawn(async move {
        if let Err(e) = kde_session_watcher_task(stop).await {
            log::error!("Error in KDE session watcher task: {}", e);
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "This test requires a KDE session to run"]
    async fn test_kde_session_monitor() {
        if !is_kde_session_watcher_available().await.unwrap_or(false) {
            log::warn!("KDE KSMServer service not available, skipping test");
            return;
        }

        let stop = OnceSignal::new();
        let monitor_stop = stop.clone();
        log::setup_logging("debug", log::LogType::Tests);
        create_kde_session_watcher_task(monitor_stop)
            .await
            .unwrap();

        stop.wait_timeout(std::time::Duration::from_secs(5))
            .await
            .unwrap();
    }
}
