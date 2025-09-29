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
    // Setup logging
    shared::log::setup_logging("debug", shared::log::LogType::Client);

    shared::log::info!("Starting uds-agent...");
    let platform = platform::Platform::new();

    run(platform).await; // To allow using tests
}

async fn run(platform: platform::Platform) {
    // Listener for the HTTP server
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap();

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

    let alive_task = tokio::spawn(tasks::alive::task(platform.clone()));

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

    // Ensure GUI is shutdown. If not done, and any window is open, process will hang until window is closed
    shared::gui::shutdown().await;
}

#[cfg(test)]
mod tests {
    // Fake api to test run function
    use crate::rest::{api::ClientRest, types::LoginResponse};
    struct FakeApi {}

    #[async_trait::async_trait]
    impl ClientRest for FakeApi {
        async fn register(&mut self, _callback_url: &str) -> Result<(), reqwest::Error> {
            Ok(())
        }
        async fn unregister(&mut self) -> Result<(), reqwest::Error> {
            Ok(())
        }
        async fn login(
            &mut self,
            _username: &str,
            _session_type: Option<&str>,
        ) -> Result<LoginResponse, reqwest::Error> {
            Ok(LoginResponse {
                ip: "127.0.0.1".into(),
                hostname: "localhost".into(),
                deadline: Some(10000),
                max_idle: Some(350),
                session_id: "sessid".into(),
            })
        }
        async fn logout(
            &self,
            _username: &str,
            _session_type: Option<&str>,
        ) -> Result<(), reqwest::Error> {
            Ok(())
        }
        async fn ping(&self) -> Result<bool, reqwest::Error> {
            Ok(true)
        }
    }

    #[tokio::test]
    async fn test_run_no_server() {
        // With no server listening, just test that run starts and stops correctly
        shared::log::setup_logging("debug", shared::log::LogType::Tests);
        // Execute run function. As long as there is no server running on localhost, it will fail to login (Before registering)
        let platform = crate::platform::Platform::new();
        let session_manager = platform.session_manager();

        let res =
            tokio::time::timeout(std::time::Duration::from_secs(4), super::run(platform)).await;
        shared::log::info!("Run finished with result: {:?}", res);

        // Stop should not be set, as run should fail to login and stop the session
        assert!(session_manager.is_running().await);
    }

    #[tokio::test]
    async fn test_run() {
        shared::log::setup_logging("debug", shared::log::LogType::Tests);
        // Start a mock server to allow login
        let api = std::sync::Arc::new(tokio::sync::RwLock::new(FakeApi {}));
        let platform = crate::platform::Platform::new_with_params(None, Some(api), None, None);

        let session_manager = platform.session_manager();

        // Run on a separate task to be able to stop it, but use a timeout to avoid hanging forever
        let run_handle = tokio::spawn(async move {
            let res =
                tokio::time::timeout(std::time::Duration::from_secs(8), super::run(platform)).await;
            shared::log::info!("Run finished with result: {:?}", res);
        });

        // Wait a bit to ensure run has started and logged in
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert!(session_manager.is_running().await);
        // Now stop the session
        session_manager.stop().await;
        shared::log::info!("Session stop requested");
        // Wait for run to finish
        let _ = run_handle.await;
        assert!(!session_manager.is_running().await);

    }
}
