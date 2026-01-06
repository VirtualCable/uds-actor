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
use reqwest::{Client, ClientBuilder, retry};
use serde::{Deserialize, Serialize};

use crate::log;

pub mod consts;
pub mod types;

pub mod block;

use anyhow::Result;
use async_trait::async_trait;

use crate::tls::CertificateInfo;

/// Trait that contains the public API methods of BrokerApi (everything except `new`)
#[async_trait]
pub trait BrokerApi: Send + Sync {
    fn clear_headers(&mut self);
    fn set_header(&mut self, _key: &str, _value: &str);

    fn get_secret(&self) -> Result<&str, types::RestError>;

    fn set_token(&mut self, token: &str);

    async fn enumerate_authenticators(&self)
    -> Result<Vec<types::Authenticator>, types::RestError>;

    async fn api_login(
        &self,
        auth: &str,
        username: &str,
        password: &str,
    ) -> Result<String, types::RestError>;

    async fn register(&self, info: &types::RegisterRequest) -> Result<String, types::RestError>;

    async fn initialize(
        &self,
        interfaces: &[crate::system::NetworkInterface],
    ) -> Result<types::InitializationResponse, types::RestError>;

    async fn ready(&self, ip: &str, port: u16) -> Result<CertificateInfo, types::RestError>;

    async fn unmanaged_ready(
        &self,
        interfaces: &[crate::system::NetworkInterface],
        port: u16,
    ) -> Result<CertificateInfo, types::RestError>;

    // Note: This is not used anymore
    // It's cleaner to stop the service and let the system (systemd, launchd, Windows service manager)
    // restart it, so the new IP is picked up cleanly and notified via ready/unmanaged_ready
    async fn notify_new_ip(&self, ip: &str, port: u16)
    -> Result<CertificateInfo, types::RestError>;

    async fn login(
        &self,
        interfaces: &[crate::system::NetworkInterface],
        username: &str,
        session_type: &str,
    ) -> Result<types::LoginResponse, types::RestError>;

    async fn logout(
        &self,
        interfaces: &[crate::system::NetworkInterface],
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
    custom_headers: reqwest::header::HeaderMap,
    // For retries
    retries: u8,
    initial_backoff: std::time::Duration,
}

impl UdsBrokerApi {
    pub fn new(
        cfg: crate::config::ActorConfiguration,
        skip_proxy: bool,
        timeout: Option<std::time::Duration>,
    ) -> Self {
        let policy = retry::for_host(cfg.broker_url.clone()).max_retries_per_request(5);

        let mut builder = ClientBuilder::new()
            .use_rustls_tls() // Use rustls for TLS
            .retry(policy)
            .timeout(timeout.unwrap_or(std::time::Duration::from_secs(2)))
            .connection_verbose(cfg!(debug_assertions))
            .danger_accept_invalid_certs(!cfg.verify_ssl);

        if skip_proxy {
            builder = builder.no_proxy();
        }

        // panic if client cannot be built, as this is a programming error (invalid URL, etc)
        let client = builder
            .build()
            .unwrap();

        // Generate a secret using random rand crate
        let rng = rand::rng();
        let secret = Some(
            rng.sample_iter(&rand::distr::Alphanumeric)
                .take(32)
                .map(char::from)
                .collect(),
        );
        let api_url = cfg.broker_url.clone();
        let actor_type = cfg.actor_type.clone();

        Self {
            api_url,
            client,
            secret,
            token: Some(cfg.token().clone()),
            actor_type,
            custom_headers: reqwest::header::HeaderMap::new(),
            retries: 3,
            initial_backoff: std::time::Duration::from_millis(500),
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
        // Add custom headers
        for (key, value) in self.custom_headers.iter() {
            headers.insert(key, value.clone());
        }
        headers
    }

    pub fn set_retry_params(&mut self, retries: u8, initial_backoff: std::time::Duration) {
        self.retries = retries;
        self.initial_backoff = initial_backoff;
    }

    fn api_url(&self, method: &str) -> String {
        // if / is on url, do not transform (already a path), else add consts::REST_ACTOR_PATH
        if method.contains('/') {
            self.api_url.clone() + method
        } else {
            self.api_url.clone() + consts::REST_ACTOR_PATH + method
        }
    }

    pub fn secret(&self) -> Option<String> {
        self.secret.clone()
    }

    async fn do_post<T: for<'de> Deserialize<'de>, P: Serialize>(
        &self,
        method: &str,
        payload: &P,
    ) -> Result<T, types::RestError> {
        log::debug!("POST to {}", self.api_url(method));

        let mut backoff = self.initial_backoff;

        for attempt in 0..=self.retries {
            let resp = self
                .client
                .post(self.api_url(method))
                .headers(self.headers())
                .json(payload)
                .send()
                .await;

            match resp {
                Ok(resp) if resp.status().is_success() => {
                    let json = resp
                        .json::<T>()
                        .await
                        .map_err(|e| types::RestError::Other(e.to_string()))?;
                    return Ok(json);
                }
                Ok(resp) => {
                    let txt = resp.text().await.unwrap_or_default();
                    return Err(types::RestError::Other(txt));
                }
                Err(e) if e.is_timeout() || e.is_connect() => {
                    if attempt < self.retries {
                        log::warn!("POST failed ({}), retrying in {:?}...", e, backoff);
                        tokio::time::sleep(backoff).await;
                        backoff = std::cmp::min(backoff * 2, std::time::Duration::from_secs(8));
                        continue;
                    } else {
                        return Err(types::RestError::Connection(e.to_string()));
                    }
                }
                Err(e) => return Err(types::RestError::Connection(e.to_string())),
            }
        }
        unreachable!()
    }

    async fn do_get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<T, types::RestError> {
        log::debug!("GET to {}", url);

        let mut backoff = self.initial_backoff;
        for attempt in 0..=self.retries {
            let resp = self
                .client
                .get(self.api_url(url))
                .headers(self.headers())
                .send()
                .await;

            log::debug!("GET response: {:?}", resp);

            match resp {
                Ok(resp) if resp.status().is_success() => {
                    let json = resp
                        .json::<T>()
                        .await
                        .map_err(|e| types::RestError::Other(e.to_string()))?;
                    return Ok(json);
                }
                Ok(resp) => {
                    let txt = resp.text().await.unwrap_or_default();
                    return Err(types::RestError::Other(txt));
                }
                Err(e) if e.is_timeout() || e.is_connect() => {
                    log::warn!("GET attempt {} failed: {}", attempt + 1, e);
                    if attempt < self.retries {
                        log::warn!("GET failed ({}), retrying in {:?}...", e, backoff);
                        tokio::time::sleep(backoff).await;
                        backoff = std::cmp::min(backoff * 2, std::time::Duration::from_secs(8));
                        continue;
                    } else {
                        return Err(types::RestError::Connection(e.to_string()));
                    }
                }
                Err(e) => return Err(types::RestError::Connection(e.to_string())),
            }
        }

        unreachable!()
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

    // Will be overriden on first call to initialize
    // on unmanaged, this is on every call, and managed on first call only
    // because master_token will be replaced by own_token
    fn set_token(&mut self, token: &str) {
        self.token = Some(token.to_string());
    }

    fn clear_headers(&mut self) {
        self.custom_headers.clear();
    }

    fn set_header(&mut self, key: &str, value: &str) {
        if let Ok(header_value) = reqwest::header::HeaderValue::from_str(value)
            && let Ok(header_name) = reqwest::header::HeaderName::from_bytes(key.as_bytes())
        {
            self.custom_headers.insert(header_name, header_value);
        }
    }

    async fn enumerate_authenticators(
        &self,
    ) -> Result<Vec<types::Authenticator>, types::RestError> {
        // GET on "auth/auths"
        let response: Vec<types::Authenticator> = self.do_get("auth/auths").await?;

        Ok(response)
    }

    /// Log in to the API
    async fn api_login(
        &self,
        auth: &str, // Auhthenticator name
        username: &str,
        password: &str,
    ) -> Result<String, types::RestError> {
        let auth_info = types::ApiLoginRequest {
            auth,
            username,
            password,
        };
        let response: types::ApiLoginResponse = self.do_post("auth/login", &auth_info).await?;
        Ok(response.token)
    }

    async fn register(&self, info: &types::RegisterRequest) -> Result<String, types::RestError> {
        // Now, register
        let response: types::ApiResponse<String> = self.do_post("register", &info).await?;
        response.result()
    }

    async fn initialize(
        &self,
        interfaces: &[crate::system::NetworkInterface],
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

    async fn ready(&self, ip: &str, port: u16) -> Result<CertificateInfo, types::RestError> {
        let payload = types::ReadyRequest {
            token: &self.get_token()?,
            secret: self.get_secret()?,
            ip,
            port,
        };

        let response: types::ApiResponse<CertificateInfo> = self.do_post("ready", &payload).await?;
        response.result()
    }

    async fn unmanaged_ready(
        &self,
        interfaces: &[crate::system::NetworkInterface],
        port: u16,
    ) -> Result<CertificateInfo, types::RestError> {
        let payload = types::UnmanagedReadyRequest {
            id: interfaces.iter().cloned().map(Into::into).collect(),
            token: &self.get_token()?,
            secret: self.get_secret()?,
            port,
        };

        let response: types::ApiResponse<CertificateInfo> =
            self.do_post("unmanaged", &payload).await?;
        response.result()
    }

    async fn notify_new_ip(
        &self,
        ip: &str,
        port: u16,
    ) -> Result<CertificateInfo, types::RestError> {
        let payload = types::ReadyRequest {
            token: &self.get_token()?,
            secret: self.get_secret()?,
            ip,
            port,
        };

        let response: types::ApiResponse<CertificateInfo> =
            self.do_post("ipchange", &payload).await?;
        response.result()
    }

    async fn login(
        &self,
        interfaces: &[crate::system::NetworkInterface],
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
        interfaces: &[crate::system::NetworkInterface],
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

#[cfg(test)]
mod tests;
