use anyhow::Result;

use futures_util::StreamExt;
use zbus::proxy::Builder;
use zbus::{Connection, Proxy};

use crate::{log, sync::OnceSignal};

#[allow(dead_code)]
pub async fn session_watch(stop: OnceSignal) -> Result<()> {
    let connection = Connection::system().await?;

    // Manager proxy
    let proxy_manager: Proxy<'_> = Builder::new(&connection)
        .destination("org.freedesktop.login1")?
        .path("/org/freedesktop/login1")?
        .interface("org.freedesktop.login1.Manager")?
        .build()
        .await?;

    // SessionRemoved signal
    let mut removed_stream = proxy_manager.receive_signal("SessionRemoved").await?;
    tokio::spawn(async move {
        log::debug!("Listening for SessionRemoved signals");
        while let Some(msg) = removed_stream.next().await {
            log::debug!("SessionRemoved signal received");
            if let Ok((id, _user, _seat)) = msg.body().deserialize::<(String, u32, String)>() {
                log::debug!("Session {} removed", id);
                stop.set();
            }
        }
    });

    // Wait a bit
    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
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
        session_watch(monitor_stop).await.unwrap();

        // Wait for a while to see if any signals are received
        tokio::time::sleep(std::time::Duration::from_secs(30)).await
    }
}
