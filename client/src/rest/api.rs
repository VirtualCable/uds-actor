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

use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;

use super::types::*;

use shared::{debug_dev, log::debug};

/// Trait that abstracts the REST client functionality for easier testing.
#[allow(dead_code)]
#[async_trait]
pub trait ClientRest: Send + Sync {
    async fn register(&mut self, callback_url: &str) -> Result<(), reqwest::Error>;
    async fn unregister(&mut self) -> Result<(), reqwest::Error>;
    async fn login(
        &mut self,
        username: &str,
        session_type: Option<&str>,
    ) -> Result<LoginResponse, reqwest::Error>;
    async fn logout(&self) -> Result<(), reqwest::Error>;
    async fn ping(&self) -> Result<bool, reqwest::Error>;
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct ClientRestSession {
    client: Client,
    base_url: String,
    session_id: String,
    callback_url: String,
    username: String,
    session_type: String,
}

#[allow(dead_code)]
impl ClientRestSession {
    /// Creates a new ClientRestApi. `base_url` should NOT include the trailing `/ui`.
    pub fn new(base_url: &str, verify_cert: bool) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(32))
            .danger_accept_invalid_certs(!verify_cert)
            .build()
            .expect("Failed to build HTTP client");

        let base_url = format!("{}/ui/", base_url.trim_end_matches('/'));

        ClientRestSession {
            client,
            base_url,
            session_id: String::new(),
            callback_url: String::new(),
            username: String::new(),
            session_type: String::new(),
        }
    }

    fn api_url(&self, method: &str) -> String {
        format!("{}{}", self.base_url, method)
    }

    pub(super) fn set_callback_url(&mut self, url: &str) {
        self.callback_url = url.to_string();
    }

    pub(super) fn set_session_id(&mut self, id: &str) {
        self.session_id = id.to_string();
    }

    pub(super) fn set_username(&mut self, username: &str) {
        self.username = username.to_string();
    }

    pub(super) fn set_session_type(&mut self, session_type: &str) {
        self.session_type = session_type.to_string();
    }

    async fn post<T, R>(&self, method: &str, payload: &T) -> Result<R, reqwest::Error>
    where
        T: serde::Serialize + ?Sized,
        R: for<'de> serde::Deserialize<'de>,
    {
        let url = self.api_url(method);
        debug_dev!("POST to {}", url);
        let res = self
            .client
            .post(&url)
            .json(payload)
            .send()
            .await?
            .json()
            .await?;
        Ok(res)
    }
}

#[async_trait]
impl ClientRest for ClientRestSession {
    async fn register(&mut self, callback_url: &str) -> Result<(), reqwest::Error> {
        debug!("Registering with callback URL: {}", callback_url);
        self.set_callback_url(callback_url);
        let payload = RegisterRequest {
            callback_url: self.callback_url.clone(),
        };
        let _: String = self.post("register", &payload).await?;
        Ok(())
    }

    async fn unregister(&mut self) -> Result<(), reqwest::Error> {
        debug!("Unregistering with callback URL: {}", self.callback_url);
        let payload = UnregisterRequest {
            callback_url: self.callback_url.clone(),
        };
        let _: String = self.post("unregister", &payload).await?;
        self.callback_url.clear();
        Ok(())
    }

    async fn login(
        &mut self,
        username: &str,
        session_type: Option<&str>,
    ) -> Result<LoginResponse, reqwest::Error> {
        debug!("Logging in user: {}", username);
        let payload = LoginRequest {
            username: username.to_string(),
            session_type: session_type.unwrap_or("UNKNOWN").to_string(),
        };
        let result: LoginResponse = self.post("login", &payload).await?;
        self.set_session_id(&result.session_id);
        self.set_username(username);
        self.set_session_type(session_type.unwrap_or("UNKNOWN"));
        debug!("Login response: {:?}", result);

        Ok(result)
    }

    async fn logout(&self) -> Result<(), reqwest::Error> {
        debug!("Logging out user: {}", self.username);
        let payload = LogoutRequest {
            username: self.username.clone(),
            session_type: self.session_type.clone(),
            callback_url: self.callback_url.clone(),
            session_id: self.session_id.clone(),
        };
        let _: String = self.post("logout", &payload).await?;
        Ok(())
    }

    async fn ping(&self) -> Result<bool, reqwest::Error> {
        debug!("Pinging server...");
        let payload = PingRequest::default();
        let result: PingResponse = self.post("ping", &payload).await?;
        debug!("Ping response: {:?}", result);
        Ok(result.0 == "pong")
    }
}

/// Create a new shared, thread-safe client implementing `ClientRest`.
#[allow(dead_code)]
pub fn new_client_rest_api() -> Arc<tokio::sync::RwLock<dyn ClientRest>> {
    Arc::new(tokio::sync::RwLock::new(ClientRestSession::new(
        "https://127.0.0.1:43910",
        false,
    )))
}
