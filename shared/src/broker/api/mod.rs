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
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::log;

pub mod consts;
pub mod types;

use anyhow::Result;

/// Client for REST API
pub struct BrokerApi {
    api_url: String,
    client: Client,
    secret: Option<String>,
    token: Option<String>,
    actor_type: crate::config::ActorType,
}

#[allow(dead_code)]
impl BrokerApi {
    pub fn new(
        api_url: &str,
        verify_ssl: bool,
        timeout: Duration,
        no_proxy: bool,
        actor_type: Option<crate::config::ActorType>,
    ) -> Self {
        let mut builder = ClientBuilder::new()
            .timeout(timeout)
            .connection_verbose(cfg!(debug_assertions))
            .danger_accept_invalid_certs(!verify_ssl);

        if no_proxy {
            builder = builder.no_proxy();
        }

        // panic if client cannot be built, as this is a programming error (invalid URL, etc)
        let client = builder
            .build()
            .map_err(|e| types::RestError::Other(e.to_string())).unwrap();

        Self {
            api_url: normalize_api_url(api_url),
            client,
            secret: None,
            token: None,
            actor_type: actor_type.unwrap_or(crate::config::ActorType::Managed),
        }
    }

    pub fn with_secret(self, secret: &str) -> Self {
        Self {
            secret: Some(secret.to_string()),
            ..self
        }
    }

    pub fn with_token(self, token: &str) -> Self {
        Self {
            token: Some(token.to_string()),
            ..self
        }
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(consts::UDS_ACTOR_AGENT).unwrap(),
        );
        headers
    }

    fn api_url(&self, method: &str) -> String {
        self.api_url.clone() + method
    }

    async fn do_post<T: for<'de> Deserialize<'de>, P: Serialize>(
        &self,
        method: &str,
        payload: &P,
    ) -> Result<T, types::RestError> {
        log::debug!("POST to {}", self.api_url(method));
        let resp = self
            .client
            .post(self.api_url(method))
            .headers(self.headers())
            .json(payload)
            .send()
            .await
            .map_err(|e| types::RestError::Connection(e.to_string()))?;

        if resp.status().is_success() {
            let json = resp
                .json::<T>()
                .await
                .map_err(|e| types::RestError::Other(e.to_string()))?;
            Ok(json)
        } else {
            let txt = resp.text().await.unwrap_or_default();
            Err(types::RestError::Other(txt))
        }
    }

    pub fn set_secret(&mut self, secret: &str) {
        self.secret = Some(secret.to_string());
    }

    pub fn set_token(&mut self, token: &str) {
        self.token = Some(token.to_string());
    }

    pub fn get_secret(&self) -> Result<&str, types::RestError> {
        self.secret
            .as_ref()
            .map(|s| s.as_ref())
            .ok_or_else(|| types::RestError::Other("No secret set".to_string()))
    }

    pub fn get_token(&self) -> Result<&str, types::RestError> {
        self.token
            .as_ref()
            .map(|s| s.as_ref())
            .ok_or_else(|| types::RestError::Other("No token set".to_string()))
    }

    pub async fn enumerate_authenticators(
        &self,
    ) -> Result<Vec<types::Authenticator>, types::RestError> {
        let response: types::ApiResponse<Vec<types::Authenticator>> =
            self.do_post("auth/auths", &()).await?;
        response.result()
    }

    /// Registers the actor, returns the registration token as String
    pub async fn register(
        &self,
        username: &str,
        hostname: &str,
        interface: &crate::operations::NetworkInterface,
        command: &types::RegisterCommandData,
        log_level: types::LogLevel,
        os: &str,
    ) -> Result<String, types::RestError> {
        let payload = types::RegisterRequest {
            version: consts::VERSION,
            build: consts::BUILD,
            username,
            hostname,
            ip: &interface.ip_addr,
            mac: &interface.mac,
            command: command.clone(),
            log_level,
            os,
        };

        // Returns the registration token as string
        let response: types::ApiResponse<String> = self.do_post("register", &payload).await?;
        response.result()
    }

    pub async fn initialize(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
    ) -> Result<types::InitializationResponse, types::RestError> {
        let payload = types::InitializationRequest {
            actor_type: self.actor_type.clone(),
            token: self.get_token()?,
            version: consts::VERSION,
            build: consts::BUILD,
            id: interfaces.iter().cloned().map(Into::into).collect(),
        };

        let response: types::ApiResponse<types::InitializationResponse> =
            self.do_post("initialize", &payload).await?;
        response.result()
    }

    pub async fn ready(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<types::CertificateInfo, types::RestError> {
        let payload = types::ReadyRequest {
            token: self.get_token()?,
            secret: self.get_secret()?,
            ip,
            port,
        };

        let response: types::ApiResponse<types::CertificateInfo> =
            self.do_post("ready", &payload).await?;
        response.result()
    }

    // Notifies a ready from an unmanaged actor
    // Data passed in are used for callbacks
    // That will be 'https://{ip}:{port}/actor/{secret}
    pub async fn unmanaged_ready(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
        port: u16,
    ) -> Result<types::CertificateInfo, types::RestError> {
        let payload = types::UnmanagedReadyRequest {
            id: interfaces.iter().cloned().map(Into::into).collect(),
            token: self.get_token()?,
            secret: self.get_secret()?,
            port,
        };

        let response: types::ApiResponse<types::CertificateInfo> =
            self.do_post("unmanaged", &payload).await?;
        response.result()
    }

    pub async fn notify_new_ip(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<types::CertificateInfo, types::RestError> {
        let payload = types::ReadyRequest {
            token: self.get_token()?,
            secret: self.get_secret()?,
            ip,
            port,
        };

        let response: types::ApiResponse<types::CertificateInfo> =
            self.do_post("ipchange", &payload).await?;
        response.result()
    }

    pub async fn login(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
        username: &str,
        session_type: &str,
    ) -> Result<types::LoginResponse, types::RestError> {
        let payload = types::LoginRequest {
            actor_type: self.actor_type.clone(),
            id: interfaces.iter().cloned().map(Into::into).collect(),
            token: self.get_token()?,
            username,
            session_type,
        };

        let response: types::ApiResponse<types::LoginResponse> =
            self.do_post("login", &payload).await?;
        response.result()
    }

    pub async fn logout(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
        username: &str,
        session_type: &str,
        session_id: &str,
    ) -> Result<String, types::RestError> {
        let payload = types::LogoutRequest {
            actor_type: self.actor_type.clone(),
            id: interfaces.iter().cloned().map(Into::into).collect(),
            token: self.get_token()?,
            username,
            session_type,
            session_id,
        };

        let response: types::ApiResponse<String> = self.do_post("logout", &payload).await?;
        response.result()
    }

    /// Sends a log message to the server
    /// Returns "ok" if successful (basically, if no Error, it's ok)
    pub async fn log(
        &self,
        level: types::LogLevel,
        message: &str,
    ) -> Result<String, types::RestError> {
        let payload = types::LogRequest {
            token: self.get_token()?,
            level,
            message,
            timestamp: chrono::Utc::now().timestamp(),
        };

        let response: types::ApiResponse<String> = self.do_post("log", &payload).await?;
        response.result()
    }

    /// Tests connectivity and authentication with the server
    /// Returns "ok" if successful (basically, if no Error, it's ok)
    pub async fn test(&self) -> Result<String, types::RestError> {
        let payload = types::TestRequest {
            actor_type: self.actor_type.clone(),
            token: self.get_token()?,
        };

        let response: types::ApiResponse<String> = self.do_post("test", &payload).await?;
        response.result()
    }
}

pub fn normalize_api_url(api_url: &str) -> String {
    // If api_url ends with a /, we assume it is a full path already
    if !api_url.ends_with('/') {
        format!("{}/{}", api_url, consts::UDS_ACTOR_ENDPOINT)
    } else {
        api_url.to_string()
    }
}

#[cfg(test)]
mod tests;
