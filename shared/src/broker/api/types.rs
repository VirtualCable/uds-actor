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

use crate::config::ActorOsConfiguration;

/// Possible errors in REST operations
#[derive(Debug)]
pub enum RestError {
    Connection(String),
    Other(String),
}

/// Actor types
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorType {
    Managed,
    Unmanaged,
}

// ************
//   Requests
// ************
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
    pub command: RegisterCommandData,
    pub log_level: LogLevel,
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
    pub name: String,
    pub mac: String,
    pub ip: String,
}

impl InterfaceInfo {
    /// Check if this interface's IP is inside the given subnet (IPv4 or IPv6).
    pub fn in_subnet(&self, subnet: Option<&str>) -> bool {
        // If no subnet provided, always valid
        let Some(subnet_str) = subnet else {
            return true;
        };

        // Try to parse subnet
        let Ok(net) = subnet_str.parse::<ipnetwork::IpNetwork>() else {
            return true; // if subnet invalid, treat as "no filter"
        };

        // Try to parse interface IP
        match self.ip.parse::<std::net::IpAddr>() {
            Ok(addr) => net.contains(addr),
            Err(_) => false,
        }
    }
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

#[derive(Debug, Clone, Serialize)]
pub struct RegisterCommandData {
    pub pre_command: Option<String>,
    pub runonce_command: Option<String>,
    pub post_command: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientInfo {
    pub url: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CertificateInfo {
    pub key: String,
    pub certificate: String,
    pub password: String,
    pub ciphers: String,
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

impl From<i32> for LogLevel {
    fn from(value: i32) -> Self {
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

impl From<LogLevel> for i32 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_interface_in_subnet() {
        let iface = InterfaceInfo {
            name: "eth0".to_string(),
            mac: "00:11:22:33:44:55".to_string(),
            ip: "192.168.1.10".to_string(),
        };
        assert!(iface.in_subnet(Some("192.168.1.0/24")));
        assert!(!iface.in_subnet(Some("192.168.2.0/24")));
    }

    #[test]
    fn test_multiple_interfaces_in_subnet() {
        let ifaces = vec![
            InterfaceInfo {
                name: "eth0".to_string(),
                mac: "00:11:22:33:44:55".to_string(),
                ip: "192.168.1.10".to_string(),
            },
            InterfaceInfo {
                name: "eth1".to_string(),
                mac: "00:11:22:33:44:56".to_string(),
                ip: "192.168.1.11".to_string(),
            },
            InterfaceInfo {
                name: "eth2".to_string(),
                mac: "00:11:22:33:44:57".to_string(),
                ip: "192.168.1.12".to_string(),
            },
            // Not in subnet
            InterfaceInfo {
                name: "eth3".to_string(),
                mac: "00:11:22:33:44:58".to_string(),
                ip: "192.168.2.10".to_string(),
            },
        ];
        let in_subnet: Vec<_> = ifaces
            .iter()
            .filter(|iface| iface.in_subnet(Some("192.168.1.0/24")))
            .collect();
        assert_eq!(in_subnet.len(), 3);
        let not_in_subnet: Vec<_> = ifaces
            .iter()
            .filter(|iface| !iface.in_subnet(Some("192.168.1.0/24")))
            .collect();
        assert_eq!(not_in_subnet.len(), 1);
    }
}
