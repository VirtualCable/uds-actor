use anyhow::Result;

use futures_util::StreamExt;
use zbus::proxy::Builder;
use zbus::{Connection, Proxy, zvariant::ObjectPath};

use crate::{log, sync::OnceSignal};

use super::session::current_session_id;

#[allow(dead_code)]
pub async fn start_session_watch_task(stop: OnceSignal) -> Result<()> {
    let connection = Connection::system().await?;

    // Manager proxy
    let proxy_manager: Proxy<'_> = Builder::new(&connection)
        .destination("org.freedesktop.login1")?
        .path("/org/freedesktop/login1")?
        .interface("org.freedesktop.login1.Manager")?
        .build()
        .await?;

    let session_id = current_session_id()?;
    if session_id.is_empty() {
        log::warn!("No current session ID found, cannot monitor session signals");
        return Ok(());
    }
    log::debug!("Current session ID: {}", session_id);

    // SessionRemoved signal
    // Note that all sessions are monitored, not just the current one
    // For testing, we can open another VT, or ssh session and close it
    // Unfortunately, this is not valid for xrdp if our app runs inside the xrdp session
    // because the session kills our process directly without going through login1
    let mut session_removed_signal = proxy_manager.receive_signal("SessionRemoved").await?;
    tokio::spawn(async move {
        log::debug!("Listening for SessionRemoved signals");
        while let Some(msg) = session_removed_signal.next().await {
            log::debug!("SessionRemoved signal received: {:?}", msg);
            if let Ok((id, _path)) = msg.body().deserialize::<(String, ObjectPath)>() {
                log::debug!("SessionRemoved: id={} path={}", id, _path);
                if id == session_id {
                    log::info!("Current session {} has been removed, stopping monitor", id);
                    stop.set();
                }
            }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "This test requires a graphical session to run"]
    async fn test_dbus_session_monitor() {
        let stop = OnceSignal::new();
        let monitor_stop = stop.clone();
        log::setup_logging("debug", log::LogType::Tests);
        // This test just runs the main function for a short time to see if it works
        start_session_watch_task(monitor_stop).await.unwrap();

        // Wait for a while to see if any signals are received
        stop.wait_timeout(std::time::Duration::from_secs(30)).await.unwrap();
    }
}
