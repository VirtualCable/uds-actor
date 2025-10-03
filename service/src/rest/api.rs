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

use anyhow::Result;

use super::{consts, types};


#[allow(dead_code)]
/// Client for REST API
pub struct ServerRestSession {
    base_url: String,
    check_certificate: bool,
    client: Client,
    timeout: Duration,
}

#[allow(dead_code)]
impl ServerRestSession {
    pub fn new(
        base_url: &str,
        verify_cert: bool,
        timeout: Duration,
        no_proxy: bool,
    ) -> Result<Self, types::RestError> {
        let mut builder = ClientBuilder::new()
            .timeout(timeout)
            .connection_verbose(cfg!(debug_assertions))
            .danger_accept_invalid_certs(!verify_cert);

        if no_proxy {
            builder = builder.no_proxy();
        }

        let client = builder
            .build()
            .map_err(|e| types::RestError::Other(e.to_string()))?;

        let base_url = if base_url.ends_with('/') {
            base_url.to_string()
        } else {
            format!("{}/", base_url)
        };

        Ok(Self {
            base_url: base_url.to_string(),
            check_certificate: verify_cert,
            client,
            timeout,
        })
    }

    fn headers(&self) -> reqwest::header::HeaderMap {
        use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&format!("UDS Actor v{}/{}", consts::VERSION, consts::BUILD))
                .unwrap(),
        );
        headers
    }

    fn api_url(&self, method: &str) -> String {
        self.base_url.clone() + method
    }

    async fn do_post<T: for<'de> Deserialize<'de>, P: Serialize>(
        &self,
        method: &str,
        payload: &P,
    ) -> Result<T, types::RestError> {
        shared::debug_dev!("POST to {}", self.api_url(method));
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

    pub async fn initialize(
        &self,
        token: &str,
        interfaces: &[types::InterfaceInfo],
        actor_type: Option<&str>,
    ) -> Result<types::InitializationResult, types::RestError> {
        #[derive(Serialize)]
        struct Payload<'a> {
            #[serde(rename = "type")]
            actor_type: &'a str,
            token: &'a str,
            version: &'a str,
            build: &'a str,
            id: Vec<types::InterfaceInfo>,
        }

        let payload = Payload {
            actor_type: actor_type.unwrap_or(consts::MANAGED),
            token,
            version: consts::VERSION,
            build: consts::BUILD,
            id: interfaces.to_vec(),
        };

        let response: types::ApiResponse<types::InitializationResult> = self.do_post("initialize", &payload).await?;
        response.result()
    }
}
