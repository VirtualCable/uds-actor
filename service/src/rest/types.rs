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
use serde::{Deserialize, Serialize};

/// Possible errors in REST operations
#[derive(Debug)]
pub enum RestError {
    Connection(String),
    Other(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct InterfaceInfo {
    pub name: String,
    pub mac: String,
    pub ip: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Authenticator {
    pub auth_id: String,
    pub auth_small_name: String,
    pub auth: String,
    pub auth_type: String,
    pub priority: i32,
    pub is_custom: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActorOsConfigurationType {
    pub action: String,
    pub name: String,
    pub custom: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActorDataConfigurationType {
    pub unique_id: Option<String>,
    pub os: Option<ActorOsConfigurationType>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ActorConfigurationType {
    pub host: String,
    pub check_certificate: bool,
    pub actor_type: Option<String>,
    pub master_token: Option<String>, // Configured master token. Will be replaced by unique one if unmanaged
    pub own_token: Option<String>, // On unmanaged, master_token will be cleared and this will be used (unique provided by server)
    pub restrict_net: Option<String>,
    pub pre_command: Option<String>,
    pub runonce_command: Option<String>,
    pub post_command: Option<String>,
    pub log_level: i32,
    pub config: Option<ActorDataConfigurationType>,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InitializationResult {
    pub master_token: Option<String>, // New master token (if unmanaged, this will be unique, may be same as provided)
    pub token: Option<String>, // For managed only. Will replace master_token by a new unique token provided by server
    pub unique_id: Option<String>, // Unique ID assigned by server to this
    pub os: Option<ActorOsConfigurationType>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginResultInfo {
    pub ip: String,
    pub hostname: String,
    pub deadline: Option<i64>,
    pub max_idle: Option<i64>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientInfo {
    pub url: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CertificateInfo {
    pub private_key: String,
    pub server_certificate: String,
    pub password: String,
    pub ciphers: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiResponse<T> {
    pub result: T,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    // If error is some and not empty, return Err
    pub fn is_error(&self) -> bool {
        if let Some(err) = &self.error {
            !err.is_empty()
        } else {
            false
        }
    }

    // Return the error as a reqwest::Error (using a generic error for demonstration)
    pub fn error(&self) -> RestError {
        RestError::Other(self.error.clone().unwrap_or_default())
    }

    pub fn result(self) -> anyhow::Result<T, RestError> {
        if self.is_error() {
            Err(self.error())
        } else {
            Ok(self.result)
        }
    }
}
