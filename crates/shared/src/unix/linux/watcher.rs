use anyhow::Result;

use zbus::proxy::Builder;
use zbus::{Connection, Proxy};

use crate::{log, sync::OnceSignal};

use super::session::current_session_id;

// We cannot rely on the SessionRemoved signal: logind only emits it once the
// session scope is empty, and our own client process lives inside that scope
// (distros ship KillUserProcesses=no), so the signal would never arrive.
// Neither can we rely on signals: xrdp kills the session leader and leaves
// the rest of the scope alone. So we poll once per second:
//   - the session State property: "closing" (or the session object being
//     gone) means logout;
//   - the session Leader pid: when it dies, the session is ending
//     (xrdp-sesman terminates it exactly at session close).
// None of this touches X, so an abrupt X server death cannot kill us first.
pub async fn start_session_watch_task(stop: OnceSignal) -> Result<()> {
    let session_id = current_session_id()?;
    if session_id.is_empty() {
        log::warn!("No current session ID found, cannot monitor session state");
        return Ok(());
    }

    let connection = Connection::system().await?;

    // Manager proxy, only to resolve our session's object path
    let proxy_manager: Proxy<'_> = Builder::new(&connection)
        .destination("org.freedesktop.login1")?
        .path("/org/freedesktop/login1")?
        .interface("org.freedesktop.login1.Manager")?
        .build()
        .await?;

    let msg = proxy_manager
        .call_method("GetSession", &session_id.as_str())
        .await?;
    let body = msg.body();
    let session_path: zbus::zvariant::OwnedObjectPath = body.deserialize()?;

    let proxy_session: Proxy<'_> = Builder::new(&connection)
        .destination("org.freedesktop.login1")?
        .path(session_path)?
        .interface("org.freedesktop.login1.Session")?
        .build()
        .await?;

    let leader_pid: u32 = proxy_session.get_property("Leader").await.unwrap_or(0);

    log::info!(
        "Watching logind session {} (leader pid {}) for logout",
        session_id,
        leader_pid
    );

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            tokio::select! {
                _ = interval.tick() => {}
                _ = stop.wait() => return,
            }

            if stop.is_set() {
                return;
            }

            // Session state: "closing" means logout is in progress; an error
            // means the session object no longer exists (already removed).
            match proxy_session.get_property::<String>("State").await {
                Ok(state) if state == "closing" => {
                    log::info!("Session {} is closing, notifying logout", session_id);
                    stop.set();
                    return;
                }
                Ok(_) => {}
                Err(e) => {
                    log::info!(
                        "Session {} no longer exists ({}), notifying logout",
                        session_id,
                        e
                    );
                    stop.set();
                    return;
                }
            }

            // Leader process: xrdp terminates it exactly at session close.
            if leader_pid != 0 && !process_alive(leader_pid) {
                log::info!(
                    "Session {} leader (pid {}) is gone, notifying logout",
                    session_id,
                    leader_pid
                );
                stop.set();
                return;
            }
        }
    });

    Ok(())
}

fn process_alive(pid: u32) -> bool {
    // kill(pid, 0): 0 = alive, EPERM = alive but owned by someone else,
    // ESRCH = gone.
    if unsafe { libc::kill(pid as libc::pid_t, 0) } == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
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

    #[test]
    fn test_process_alive() {
        assert!(process_alive(std::process::id()));
        // Pid 0x7FFFFFFF should not exist
        assert!(!process_alive(0x7FFFFFFF));
    }
}
