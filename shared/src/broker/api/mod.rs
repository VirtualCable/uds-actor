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
use rand::prelude::*;
use reqwest::{Client, ClientBuilder};
use serde::{Deserialize, Serialize};

use crate::log;

pub mod consts;
pub mod types;

pub mod sync;

use anyhow::Result;
use async_trait::async_trait;

/// Trait that contains the public API methods of BrokerApi (everything except `new`)
#[async_trait]
pub trait BrokerApi: Send + Sync {
    fn get_secret(&self) -> Result<&str, types::RestError>;

    async fn enumerate_authenticators(&self)
    -> Result<Vec<types::Authenticator>, types::RestError>;

    async fn register(
        &self,
        username: &str,
        hostname: &str,
        interface: &crate::operations::NetworkInterface,
        command: &types::RegisterCommandData,
        log_level: types::LogLevel,
        os: &str,
    ) -> Result<String, types::RestError>;

    async fn initialize(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
    ) -> Result<types::InitializationResponse, types::RestError>;

    async fn ready(&self, ip: &str, port: u16) -> Result<types::CertificateInfo, types::RestError>;

    async fn unmanaged_ready(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
        port: u16,
    ) -> Result<types::CertificateInfo, types::RestError>;

    async fn notify_new_ip(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<types::CertificateInfo, types::RestError>;

    async fn login(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
        username: &str,
        session_type: &str,
    ) -> Result<types::LoginResponse, types::RestError>;

    async fn logout(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
        username: &str,
        session_type: &str,
        session_id: &str,
    ) -> Result<String, types::RestError>;

    async fn log(&self, level: types::LogLevel, message: &str) -> Result<String, types::RestError>;

    async fn test(&self) -> Result<String, types::RestError>;
}

/// Client for REST API
pub struct UdsBrokerApi {
    client: Client,
    api_url: String,
    secret: Option<String>,
    token: Option<String>,
    actor_type: crate::config::ActorType,
}

#[allow(dead_code)]
impl UdsBrokerApi {
    pub fn new(
        cfg: crate::config::ActorConfiguration,
    ) -> Self {
        let mut builder = ClientBuilder::new()
            .timeout(cfg.timeout())
            .connection_verbose(cfg!(debug_assertions))
            .danger_accept_invalid_certs(!cfg.verify_ssl);

        if cfg.no_proxy {
            builder = builder.no_proxy();
        }

        // panic if client cannot be built, as this is a programming error (invalid URL, etc)
        let client = builder
            .build()
            .map_err(|e| types::RestError::Other(e.to_string()))
            .unwrap();

        // Generate a secret using random rand crate
        let rng = rand::rng();
        let secret = Some(
            rng.sample_iter(&rand::distr::Alphanumeric)
                .take(32)
                .map(char::from)
                .collect(),
        );
        let api_url = normalize_api_url(cfg.broker_url.as_str());

        Self {
            api_url,
            client,
            secret,
            token: Some(cfg.token().clone()),
            actor_type: cfg.actor_type.unwrap()
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

    fn secret(&self) -> Option<String> {
        self.secret.clone()
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

    pub fn get_token(&self) -> Result<String, types::RestError> {
        let token = self.token.clone();
        token.ok_or_else(|| types::RestError::Other("No token set".to_string()))
    }

    pub fn actor_type(&self) -> crate::config::ActorType {
        self.actor_type.clone()
    }
}

#[async_trait]
impl BrokerApi for UdsBrokerApi {
    fn get_secret(&self) -> Result<&str, types::RestError> {
        self.secret
            .as_ref()
            .map(|s| s.as_ref())
            .ok_or_else(|| types::RestError::Other("No secret set".to_string()))
    }

    async fn enumerate_authenticators(
        &self,
    ) -> Result<Vec<types::Authenticator>, types::RestError> {
        let response: types::ApiResponse<Vec<types::Authenticator>> =
            self.do_post("auth/auths", &()).await?;
        response.result()
    }

    async fn register(
        &self,
        username: &str,
        hostname: &str,
        interface: &crate::operations::NetworkInterface,
        command: &types::RegisterCommandData,
        log_level: types::LogLevel,
        os: &str,
    ) -> Result<String, types::RestError> {
        let payload = types::RegisterRequest {
            version: crate::consts::VERSION,
            build: crate::consts::BUILD,
            username,
            hostname,
            ip: &interface.ip_addr,
            mac: &interface.mac,
            command: command.clone(),
            log_level,
            os,
        };

        let response: types::ApiResponse<String> = self.do_post("register", &payload).await?;
        response.result()
    }

    async fn initialize(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
    ) -> Result<types::InitializationResponse, types::RestError> {
        let payload = types::InitializationRequest {
            actor_type: self.actor_type(),
            token: &self.get_token()?,
            version: crate::consts::VERSION,
            build: crate::consts::BUILD,
            id: interfaces.iter().cloned().map(Into::into).collect(),
        };

        let response: types::ApiResponse<types::InitializationResponse> =
            self.do_post("initialize", &payload).await?;
        response.result()
    }

    async fn ready(&self, ip: &str, port: u16) -> Result<types::CertificateInfo, types::RestError> {
        let payload = types::ReadyRequest {
            token: &self.get_token()?,
            secret: self.get_secret()?,
            ip,
            port,
        };

        let response: types::ApiResponse<types::CertificateInfo> =
            self.do_post("ready", &payload).await?;
        response.result()
    }

    async fn unmanaged_ready(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
        port: u16,
    ) -> Result<types::CertificateInfo, types::RestError> {
        let payload = types::UnmanagedReadyRequest {
            id: interfaces.iter().cloned().map(Into::into).collect(),
            token: &self.get_token()?,
            secret: self.get_secret()?,
            port,
        };

        let response: types::ApiResponse<types::CertificateInfo> =
            self.do_post("unmanaged", &payload).await?;
        response.result()
    }

    async fn notify_new_ip(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<types::CertificateInfo, types::RestError> {
        let payload = types::ReadyRequest {
            token: &self.get_token()?,
            secret: self.get_secret()?,
            ip,
            port,
        };

        let response: types::ApiResponse<types::CertificateInfo> =
            self.do_post("ipchange", &payload).await?;
        response.result()
    }

    async fn login(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
        username: &str,
        session_type: &str,
    ) -> Result<types::LoginResponse, types::RestError> {
        let payload = types::LoginRequest {
            actor_type: self.actor_type(),
            id: interfaces.iter().cloned().map(Into::into).collect(),
            token: &self.get_token()?,
            username,
            session_type,
        };

        let response: types::ApiResponse<types::LoginResponse> =
            self.do_post("login", &payload).await?;
        response.result()
    }

    async fn logout(
        &self,
        interfaces: &[crate::operations::NetworkInterface],
        username: &str,
        session_type: &str,
        session_id: &str,
    ) -> Result<String, types::RestError> {
        let payload = types::LogoutRequest {
            actor_type: self.actor_type(),
            id: interfaces.iter().cloned().map(Into::into).collect(),
            token: &self.get_token()?,
            username,
            session_type,
            session_id,
        };

        let response: types::ApiResponse<String> = self.do_post("logout", &payload).await?;
        response.result()
    }

    async fn log(&self, level: types::LogLevel, message: &str) -> Result<String, types::RestError> {
        let payload = types::LogRequest {
            token: &self.get_token()?,
            level,
            message,
            timestamp: chrono::Utc::now().timestamp(),
        };

        let response: types::ApiResponse<String> = self.do_post("log", &payload).await?;
        response.result()
    }

    async fn test(&self) -> Result<String, types::RestError> {
        let payload = types::TestRequest {
            actor_type: self.actor_type(),
            token: &self.get_token()?,
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
