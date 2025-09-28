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

async fn send_logout(platform: &platform::Platform) -> anyhow::Result<()> {
    let username = platform.operations().get_current_user()?;
    let session_type = platform.operations().get_session_type()?;
    let api = platform.api();

    api.write()
        .await
        .logout(&username, Some(&session_type))
        .await
        .map_err(|e| anyhow::anyhow!("Failed to logout: {}", e))
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    shared::log::setup_logging("debug", shared::log::LogType::Client);

    let platform = platform::Platform::new();
    let server_platform = platform.clone();

    // Listener for the HTTP server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

    // Local server registers the callback, so it neesds to be started before login
    let server_task =
        tokio::spawn(async move { http::run_server(listener, server_platform).await });

    let login_info = match send_login(&platform).await {
        Ok(info) => info,
        Err(e) => {
            shared::log::error!("Login failed: {}", e);
            return;
        }
    };
    shared::log::info!("Login successful: {:?}", login_info);

    let idle_task = tokio::spawn(tasks::idle::task(login_info.max_idle, platform.clone()));

    let platform_for_deadline = platform.clone();
    let deadline_task = tokio::spawn(async move {
        tasks::deadline::task(login_info.deadline, platform_for_deadline).await
    });

    let platform_for_alive = platform.clone();
    let alive_task = tokio::spawn(async move { tasks::alive::task(platform_for_alive).await });

    let session_manager = platform.session_manager();
    // Await for session end
    session_manager.wait().await;

    send_logout(&platform).await.unwrap_or_else(|e| {
        shared::log::error!("Logout failed: {}", e);
    });

    // Join with a timeout all tasks to avoid hanging forever
    let join_timeout = std::time::Duration::from_secs(5);
    if tokio::time::timeout(join_timeout, async {
        let _ = server_task.await;
        let _ = idle_task.await;
        let _ = deadline_task.await;
        let _ = alive_task.await;
    })
    .await
    .is_err()
    {
        shared::log::warn!("Some tasks did not shut down in time, aborting...");
    }
}
