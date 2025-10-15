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
// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![cfg_attr(not(test), windows_subsystem = "windows")]

mod http;
mod rest;
mod session;

mod platform;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

// Tasks
mod tasks;

async fn send_login(platform: &platform::Platform) -> anyhow::Result<rest::types::LoginResponse> {
    // Send login
    let username = platform.operations().get_current_user()?;
    let session_type = platform.operations().get_session_type()?;
    let api = platform.api();

    api.write()
        .await
        .login(&username, Some(&session_type))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to login: {}", e))
}

async fn send_logout(platform: &platform::Platform, reason: Option<&str>) -> anyhow::Result<()> {
    let api = platform.api();

    api.write()
        .await
        .logout(reason)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to logout: {}", e))
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Setup logging
    shared::log::setup_logging("debug", shared::log::LogType::Client);

    shared::log::info!("Starting uds-actor client...");
    let platform = platform::Platform::new();

    run(platform.clone()).await; // Run main loop
    shared::log::info!("uds-actor client stopped.");

    platform.shutdown();
}

async fn run(platform: platform::Platform) {
    // Listener for the HTTP server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();

    let login_info = match send_login(&platform).await {
        Ok(info) => info,
        Err(e) => {
            shared::log::error!("Login failed: {}", e);
            return;
        }
    };
    shared::log::info!("Login successful: {:?}", login_info);

    // Local server registers the callback, so it needs to be started before login
    let server_task = tokio::spawn(http::run_server(listener, platform.clone()));

    let idle_task = tokio::spawn(tasks::idle::task(login_info.max_idle, platform.clone()));

    let deadline_task = tokio::spawn(tasks::deadline::task(login_info.deadline, platform.clone()));

    let signals_task = tokio::spawn(tasks::signals::task(platform.clone()));

    // On legacy, no ping is needed
    //let alive_task = tokio::spawn(tasks::alive::task(platform.clone()));

    let session_manager = platform.session_manager();
    // Await for session end
    session_manager.wait().await;

    // Join with a timeout all tasks to avoid hanging forever
    let join_timeout = std::time::Duration::from_secs(5);
    let mut reason = None;
    if tokio::time::timeout(join_timeout, async {
        let _ = server_task.await;
        for task in [idle_task, deadline_task, signals_task] {
            let res = task.await;
            if let Ok(Ok(Some(r))) = res {
                reason = Some(r);
                break;
            }
        }
    })
    .await
    .is_err()
    {
        shared::log::warn!("Some tasks did not shut down in time, aborting...");
    }

    // Send logout
    if let Err(e) = send_logout(&platform, reason.as_deref()).await {
        shared::log::error!("Logout failed: {}", e);
    } else {
        shared::log::info!("Logout successful");
    }

    // Ensure GUI is shutdown. If not done, and any window is open, process will hang until window is closed
}

// Dummy modules for tests
#[cfg(test)]
pub mod testing;

#[cfg(test)]
pub mod tests;
