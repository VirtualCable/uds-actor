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
use shared::{tls, ws::{
    types::{LoginRequest, LoginResponse, LogoutRequest, RpcEnvelope, RpcMessage},
    wait_message_arrival,
}};

mod session;

mod gui;
mod platform;
mod workers;

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

// Tasks
mod tasks;

async fn send_login(platform: &platform::Platform) -> anyhow::Result<LoginResponse> {
    // Send login
    let username = platform.operations().get_current_user()?;
    let session_type = platform.operations().get_session_type()?;
    let ws_client = platform.ws_client();
    let stop = platform.get_stop();

    ws_client
        .to_ws
        .send(RpcEnvelope {
            id: Some(19720701), // Some arbitrary id
            msg: RpcMessage::LoginRequest(LoginRequest {
                username: username.clone(),
                session_type: session_type.clone(),
            }),
        })
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send login message: {}", e))?;

    // Wait for response
    let mut rx = ws_client.from_ws.subscribe();
    let envelope = wait_message_arrival::<LoginResponse>(&mut rx, Some(stop))
        .await
        .ok_or_else(|| anyhow::anyhow!("Failed to receive login response for user {}", username))?;

    Ok(envelope.msg)
}

async fn send_logout(platform: &platform::Platform, session_id: Option<&str>) -> anyhow::Result<()> {
    let username = platform.operations().get_current_user()?;
    let session_type = platform.operations().get_session_type()?;
    let ws_client = platform.ws_client();

        ws_client
        .to_ws
        .send(RpcEnvelope {
            id: Some(19720701), // Some arbitrary id
            msg: RpcMessage::LogoutRequest(LogoutRequest {
                username: username.clone(),
                session_type: session_type.clone(),
                session_id: session_id.unwrap_or_default().to_string(),
            }),
        })
        .await
        .map_err(|e| anyhow::anyhow!("Failed to send logout message: {}", e))?;

    Ok(())

}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    // Setup logging
    shared::log::setup_logging("debug", shared::log::LogType::Client);
    tls::init_tls(None);

    shared::log::info!("Starting uds-actor client...");
    let platform = platform::Platform::new(43910).await;

    run(platform.clone()).await; // Run main loop
    shared::log::info!("uds-actor client stopped.");

    platform.shutdown();
}

async fn run(platform: platform::Platform) {
    let login_info = match send_login(&platform).await {
        Ok(info) => info,
        Err(e) => {
            shared::log::error!("Login failed: {}", e);
            return;
        }
    };
    shared::log::info!("Login successful: {:?}", login_info);

    // Setup ws workers
    workers::setup_workers(platform.clone()).await;

    // Monitoring tasks, can stop the app (and session itself)
    let idle_task = tokio::spawn(tasks::idle::task(login_info.max_idle, platform.clone()));
    let deadline_task = tokio::spawn(tasks::deadline::task(login_info.deadline, platform.clone()));

    // Await for session end
    platform.get_stop().wait().await;

    // Join with a timeout all tasks to avoid hanging forever
    let join_timeout = std::time::Duration::from_secs(5);
    let mut reason = None;
    if tokio::time::timeout(join_timeout, async {
        for task in [idle_task, deadline_task] {
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
