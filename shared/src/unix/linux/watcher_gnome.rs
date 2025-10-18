use anyhow::Result;
use futures_util::StreamExt;
use zbus::{Connection, proxy};

use crate::{log, sync::OnceSignal};

#[proxy(
    interface = "org.gnome.SessionManager",
    default_path = "/org/gnome/SessionManager",
    default_service = "org.gnome.SessionManager"
)]
trait SessionManager {
    // We only care about signals here
    #[zbus(signal)]
    fn EndSession(&self);
    #[zbus(signal)]
    fn QueryEndSession(&self);
    #[zbus(signal)]
    fn SessionOver(&self);
}

/// Check if GNOME SessionManager is available on the session bus.
/// Returns true if the service is present, false otherwise.
pub async fn is_gnome_session_watcher_available() -> Result<bool> {
    let conn = Connection::session().await?;

    let msg = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "NameHasOwner",
            &("org.gnome.SessionManager",),
        )
        .await?;

    let reply: bool = msg.body().deserialize()?;
    Ok(reply)
}

/// Start a background task that watches GNOME session signals.
/// It will stop either when `stop` is set or when a session end signal arrives.
async fn gnome_session_watcher_task(stop: OnceSignal) -> Result<()> {
    let conn = Connection::session().await?;
    let proxy = SessionManagerProxy::new(&conn).await?;

    let mut end_session = proxy.receive_EndSession().await?;
    let mut query_end = proxy.receive_QueryEndSession().await?;
    let mut session_over = proxy.receive_SessionOver().await?;

    log::debug!("GNOME session watch task started");
    loop {
        tokio::select! {
            _ = stop.wait() => {
                log::debug!("Stop signal received, ending GNOME session watch");
                break;
            }
            msg = end_session.next() => {
                if msg.is_some() {
                    log::info!("EndSession signal received, shutting down gracefully");
                    stop.set(); // propagate to the rest of the system
                    break;
                }
            }
            msg = query_end.next() => {
                if msg.is_some() {
                    log::info!("QueryEndSession signal received, preparing for shutdown");
                    // optional pre-cleanup
                    stop.set();
                    break;
                }
            }
            msg = session_over.next() => {
                if msg.is_some() {
                    log::warn!("SessionOver signal received, session has ended");
                    stop.set(); // make sure everyone else is notified
                    break;
                }
            }
        }
    }
    log::debug!("GNOME session watch task finished");
    Ok(())
}

pub async fn create_gnome_session_watcher_task(stop: OnceSignal) -> Result<()> {
    if !is_gnome_session_watcher_available().await? {
        log::warn!("GNOME SessionManager service not available, cannot start session watcher");
        return Err(anyhow::anyhow!(
            "GNOME SessionManager service not available"
        ));
    }

    tokio::spawn(async move {
        if let Err(e) = gnome_session_watcher_task(stop).await {
            log::error!("Error in GNOME session watcher task: {}", e);
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "This test requires a GNOME session to run"]
    async fn test_gnome_session_monitor() {
        if !is_gnome_session_watcher_available().await.unwrap_or(false) {
            log::warn!("GNOME SessionManager service not available, skipping test");
            return;
        }

        let stop = OnceSignal::new();
        let monitor_stop = stop.clone();
        log::setup_logging("debug", log::LogType::Tests);
        // This test just runs the main function for a short time to see if it works
        create_gnome_session_watcher_task(monitor_stop)
            .await
            .unwrap();

        // Wait for a while to see if any signals are received
        stop.wait_timeout(std::time::Duration::from_secs(5))
            .await
            .unwrap();
    }
}
