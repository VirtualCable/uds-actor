// Copyright (c) 2025 Virtual Cable S.L.U.
// All rights reserved.
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
//    * Redistributions of source code must retain the above copyright notice,
//      this list of conditions and the following disclaimer.
//    * Redistributions in binary form must reproduce the above copyright notice,
//      this list of conditions and the following disclaimer in the documentation
//      and/or other materials provided with the distribution.
//    * Neither the name of Virtual Cable S.L.U. nor the names of its contributors
//      may be used to endorse or promote products derived from this software
//      without specific prior written permission.
//
// THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
// AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
// IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
// DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
// FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
// DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
// SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
// CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
// OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
// OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
/*!
Author: Adolfo Gómez, dkmaster at dkmon dot com
*/
use anyhow::Result;
use std::{env, io, process::Command};
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::OwnedObjectPath;

/// Try to terminate the current session via D-Bus (sync).
fn try_dbus_logout(session_id: &str) -> Result<bool> {
    let connection = Connection::system()?; // synchronous
    let proxy = Proxy::new(
        &connection,
        "org.freedesktop.login1",
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )?;

    proxy.call_method("TerminateSession", &(session_id.to_string(),))?;
    Ok(true)
}

/// Fallback: invokes `loginctl terminate-session <id>`
fn fallback_loginctl(session_id: &str) -> Result<()> {
    Command::new("loginctl")
        .arg("terminate-session")
        .arg(session_id)
        .status()?;
    Ok(())
}

/// Logouts the user with dbus or loginctl
pub(super) fn logout() -> Result<()> {
    crate::log::debug!("Attempting to log out current session");
    let session_id = current_session_id()?;
    match try_dbus_logout(&session_id) {
        Ok(true) => {
            crate::log::debug!("Logout using D-Bus successful");
            Ok(())
        }
        Ok(false) => {
            crate::log::warn!("Logout using D-Bus not supported, falling back to loginctl");
            fallback_loginctl(&session_id)
        }
        Err(e) => {
            crate::log::warn!("D-Bus failed: {:?}, falling back to loginctl", e);
            fallback_loginctl(&session_id)
        }
    }
}

/// Intenta obtener el session id actual de varias formas (sync)
pub fn current_session_id() -> io::Result<String> {
    if let Ok(id) = env::var("XDG_SESSION_ID") {
        if !id.is_empty() {
            return Ok(id);
        }
    }

    if let Ok(connection) = Connection::system() {
        if let Ok(proxy) = Proxy::new(
            &connection,
            "org.freedesktop.login1",
            "/org/freedesktop/login1",
            "org.freedesktop.login1.Manager",
        ) {
            let pid = std::process::id();
            let (path,): (OwnedObjectPath,) =
                proxy.call("GetSessionByPID", &(pid,)).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!("D-Bus GetSessionByPID failed: {:?}", e),
                    )
                })?;
            if let Some(id) = path.to_string().rsplit('/').next() {
                return Ok(id.trim_start_matches('_').to_string());
            }
        }
    }

    let output = Command::new("loginctl")
        .arg("show-user")
        .arg(whoami::username())
        .arg("--property=Display")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(id) = stdout.split('=').nth(1) {
        let id = id.trim();
        if !id.is_empty() {
            return Ok(id.to_string());
        }
    }

    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "No session id found",
    ))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_current_session_id() {
        crate::log::setup_logging("debug", crate::log::LogType::Tests);
        let id = current_session_id().unwrap();
        crate::log::info!("Current session ID: {}", id);
        assert!(!id.is_empty());
    }
}
