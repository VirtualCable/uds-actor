// Copyright (c) 2025 Virtual Cable S.L.U.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
//
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

use std::sync::Arc;
use tokio::sync::Mutex;

use shared::{
    broker::api::BrokerApi,
    log,
    ws::{
        server::ServerContext,
        types::{LoginRequest, RpcEnvelope, RpcMessage},
        wait_message_arrival,
    },
};

use crate::{common, platform};

use super::logger::FloodGuard;

// Login on unmanaged actor is a bit different.
// On VM Start, we could not "identify" the machine with an user service, because not user service is created yet for this.
// So, we just wait for a LoginRequest from the wsclient, and then notify correctly to the broker.
// So we need to call "initialize" prior to login, to get the machine attached to the userservice.
// Also, we will no "save" the token, store it on memoy only, as unmanaged actors are not supposed to be long-lived.
pub async fn worker(server_info: ServerContext, platform: platform::Platform) -> Result<()> {
    let mut rx = server_info.from_ws.subscribe();
    // Login relay rate limit: the legitimate flow is one login per session
    // lifetime, so a generous cap (10/60s) costs nothing while bounding the
    // broker churn (login + the initialize it triggers) a peer can cause with
    // repeated (failed) logins.
    let login_limiter = Arc::new(Mutex::new(FloodGuard::with_limit(10)));
    while let Some(env) =
        wait_message_arrival::<LoginRequest>(&mut rx, Some(platform.get_stop())).await
    {
        log::debug!("Received LoginRequest with id {:?}", env.id);
        // UDS grants a single remote session per machine: a login while
        // another session is active must never happen in normal operation
        // (the session ends with an explicit logout or the channel closing,
        // both of which clear the recorded user). Ignore it, and surface it
        // through the peer-manufactured warning budget if it looks like abuse.
        if platform.get_user_info().read().await.is_some() {
            if shared::log_forward::peer_warn_allow() {
                log::warn!(
                    "LoginRequest for '{}' received while a session is already active; \
                     ignoring (possible forced-login attempt)",
                    env.msg.username
                );
            } else {
                log::debug!("LoginRequest ignored: session already active");
            }
            continue;
        }
        if !login_limiter.lock().await.allow() {
            log::debug!("LoginRequest dropped: login relay rate limited");
            continue;
        }
        let broker_api: std::sync::Arc<tokio::sync::RwLock<dyn BrokerApi>> = platform.broker_api();

        let interfaces = platform.system().get_network_info()?;

        if let Err(e) = common::initialize(&platform).await {
            log::error!("Failed to initialize unmanaged actor prior to login: {}", e);
            continue;
        }

        if let Ok(response) = broker_api
            .write()
            .await
            .login(
                interfaces.as_slice(),
                env.msg.username.as_str(),
                env.msg.session_type.as_str(),
            )
            .await
        {
            platform
                .get_user_info()
                .write()
                .await
                .replace(platform::UserInfo {
                    username: env.msg.username.clone(),
                    session_type: env.msg.session_type.clone(),
                    session_id: response.session_id.clone(),
                });
            let response_env = RpcEnvelope {
                id: env.id,
                msg: RpcMessage::LoginResponse(response),
            };
            if let Err(e) = server_info.to_ws.send(response_env).await {
                log::error!("Failed to send LoginResponse: {}", e);
            } else {
                log::debug!("Sent LoginResponse for id {:?}", env.id);
            }
        } else {
            log::error!("Login failed for user {}", env.msg.username);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::testing::mock;
    use std::time::Duration;

    #[tokio::test]
    async fn test_login_worker() {
        log::setup_logging("debug", shared::log::LogType::Tests);
        let server_info = mock::mock_server_info().await;
        let mocked_platform = mock::mock_platform().await;
        let platform = mocked_platform.platform.clone();
        let calls = mocked_platform.calls.clone();
        platform.config().write().await.master_token = Some("mastertoken".into());

        let wsclient_to_workers = server_info.from_ws.clone();
        let _handle = tokio::spawn(async move {
            worker(server_info, platform).await.unwrap();
        });

        // Wait to have at least one receiver
        while wsclient_to_workers.receiver_count() == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        log::info!("wsclient_to_workers has receiver");

        // Send 3 login requests: UDS grants a single remote session per
        // machine, so the first one wins and the rest are ignored (they
        // would be a forced-login attempt while a session is active).
        for i in 1..=3 {
            let req = RpcEnvelope {
                id: None,
                msg: RpcMessage::LoginRequest(LoginRequest {
                    username: format!("user{}", i),
                    session_type: "test".into(),
                }),
            };
            log::info!("Sending login request for user{}", i);
            wsclient_to_workers.send(req).unwrap();
        }

        // Wait a bit to let processing happen
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Inspect dummy broker_api: exactly one initialize + login (the
        // active session); the surplus logins must not reach the broker.
        log::info!("calls: {:?}", calls.dump());
        assert!(calls.count_calls("broker_api::initialize(") == 1);
        assert!(calls.count_calls("broker_api::login(") == 1);
    }
}
