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

use crate::config::{ActorOsConfiguration, ActorType};

/// Possible errors in REST operations
#[derive(Debug)]
pub enum RestError {
    Connection(String),
    Other(String),
}

// ************
//   Requests
// ************
#[derive(Debug, Serialize)]
pub struct ApiLoginRequest<'a> {
    pub auth: &'a str,
    pub username: &'a str,
    pub password: &'a str,
}

#[derive(Debug, Serialize)]
pub struct InitializationRequest<'a> {
    #[serde(rename = "type")]
    pub actor_type: ActorType,
    pub token: &'a str,
    pub version: &'a str,
    pub build: &'a str,
    pub id: Vec<InterfaceInfo>,
}

#[derive(Debug, Serialize)]
pub struct RegisterRequest<'a> {
    pub version: &'a str,
    pub build: &'a str,
    pub username: &'a str,
    pub hostname: &'a str,
    pub ip: &'a str,
    pub mac: &'a str,
    pub commands: RegisterCommands,
    pub log_level: u32,
    pub os: &'a str,
}

#[derive(Debug, Serialize)]
pub struct ReadyRequest<'a> {
    pub token: &'a str,
    pub secret: &'a str,
    pub ip: &'a str,
    pub port: u16,
}

#[derive(Debug, Serialize)]
pub struct UnmanagedReadyRequest<'a> {
    pub id: Vec<InterfaceInfo>,
    pub token: &'a str,
    pub secret: &'a str,
    pub port: u16,
}

#[derive(Debug, Serialize)]
pub struct LoginRequest<'a> {
    #[serde(rename = "type")]
    pub actor_type: ActorType,
    pub id: Vec<InterfaceInfo>,
    pub token: &'a str,
    pub username: &'a str,
    pub session_type: &'a str,
}

#[derive(Debug, Serialize)]
pub struct LogoutRequest<'a> {
    #[serde(rename = "type")]
    pub actor_type: ActorType,
    pub id: Vec<InterfaceInfo>,
    pub token: &'a str,
    pub username: &'a str,
    pub session_type: &'a str,
    pub session_id: &'a str,
}

#[derive(Debug, Serialize)]
pub struct LogRequest<'a> {
    pub token: &'a str,
    pub level: LogLevel,
    pub message: &'a str,
    pub timestamp: i64,
}

#[derive(Debug, Serialize)]
pub struct TestRequest<'a> {
    #[serde(rename = "type")]
    pub actor_type: ActorType,
    pub token: &'a str,
}

// ************
//   Responses
// ************
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiLoginResponse {
    pub result: String, // Info
    pub error: Option<String>,
    pub token: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InitializationResponse {
    pub master_token: Option<String>, // New master token (if unmanaged, this will be unique, may be same as provided)
    pub token: Option<String>, // For managed only. Will replace master_token by a new unique token provided by server
    pub unique_id: Option<String>, // Unique ID assigned by server to this
    pub os: Option<ActorOsConfiguration>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoginResponse {
    pub ip: String,
    pub hostname: String,
    pub deadline: Option<i64>,
    pub max_idle: Option<i64>,
    pub session_id: Option<String>,
}

// All responses from API are of this type
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub result: T,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
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

// ************
//    Types
// ************
#[derive(Debug, Clone, Serialize)]
pub struct InterfaceInfo {
    pub mac: String,
    pub ip: String,
}

impl From<crate::operations::NetworkInterface> for InterfaceInfo {
    fn from(iface: crate::operations::NetworkInterface) -> Self {
        InterfaceInfo {
            mac: iface.mac,
            ip: iface.ip_addr,
        }
    }
}

// TODO: On a future, use the new authenticator structure from server
// when server is updated to a version that supports it
// Note that renamed fields are already present on 5.0 servers
// But initally, we will use the old ones for compatibility with older servers
// So we can use it now on 4.x, but we will rename the fields to the new ones
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Authenticator {
    #[serde(rename = "auth_id")]  // On future releases, this will be "id"
    pub id: String,
    #[serde(rename = "auth_label")]  // On future releases, this will be "label"
    pub label: String,
    #[serde(rename = "auth")]  // On future releases, this will be "name"
    pub name: String,
    #[serde(rename = "type")]  // "type" is a reserved word, so we use "auth_type" in struct
    pub auth_type: String,
    pub priority: i32,
    pub custom: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegisterCommands {
    pub pre_command: Option<String>,
    pub runonce_command: Option<String>,
    pub post_command: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientInfo {
    pub url: String,
    pub session_id: String,
}


// Log levels, must match server ones
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum LogLevel {
    Other = 10000,
    Debug = 20000,
    Info = 30000,
    Warn = 40000,
    Error = 50000,
    Fatal = 60000,
}

// From u8, wil get from 0 to 5, where 0 is Debug and 5 is Fatal
impl From<u8> for LogLevel {
    fn from(value: u8) -> Self {
        match value {
            0 => LogLevel::Debug,
            1 => LogLevel::Info,
            2 => LogLevel::Warn,
            3 => LogLevel::Error,
            4 => LogLevel::Fatal,
            _ => LogLevel::Other,
        }
    }
}

impl From<LogLevel> for u8 {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Warn => 2,
            LogLevel::Error => 3,
            LogLevel::Fatal => 4,
            LogLevel::Other => 5,
        }
    }
}

impl From<u32> for LogLevel {
    fn from(value: u32) -> Self {
        match value {
            20000 => LogLevel::Debug,
            30000 => LogLevel::Info,
            40000 => LogLevel::Warn,
            50000 => LogLevel::Error,
            60000 => LogLevel::Fatal,
            _ => LogLevel::Other,
        }
    }
}

impl From<LogLevel> for u32 {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Debug => 20000,
            LogLevel::Info => 30000,
            LogLevel::Warn => 40000,
            LogLevel::Error => 50000,
            LogLevel::Fatal => 60000,
            LogLevel::Other => 10000,
        }
    }
}

impl From<&str> for LogLevel {
    fn from(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "debug" => LogLevel::Debug,
            "info" => LogLevel::Info,
            "warn" => LogLevel::Warn,
            "error" => LogLevel::Error,
            "fatal" => LogLevel::Fatal,
            _ => LogLevel::Other,
        }
    }
}
