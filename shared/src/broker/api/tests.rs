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
use super::*;
use super::{consts, types};

use crate::log::{self, info};

use mockito::{Matcher, Server};

// Helper to create a ServerRestApi pointing to mockito server
// Helper to create a mockito server and a ServerRestApi pointing to it
async fn setup_server_and_api() -> (mockito::ServerGuard, BrokerApi) {
    log::setup_logging("debug", log::LogType::Tests);

    info!("Setting up mock server and API client");
    let server = Server::new_async().await;
    let url = server.url() + "/"; // For testing, our base URL will be the mockito server
    // Pass the base url (without /ui) to the API
    (
        server,
        BrokerApi::new(
            &url,
            false,
            std::time::Duration::from_secs(5),
            true,
            Some(crate::config::ActorType::Managed),
        )
        .with_token("token")
        .with_secret("secret"),
    )
}

// Helper to create an id with some interfaces
fn create_test_id() -> Vec<crate::operations::NetworkInterface> {
    vec![
        crate::operations::NetworkInterface {
            name: "eth0".to_string(),
            mac: "00:11:22:33:44:55".to_string(),
            ip_addr: "192.168.1.1".to_string(),
        },
        crate::operations::NetworkInterface {
            name: "wlan0".to_string(),
            mac: "66:77:88:99:AA:BB".to_string(),
            ip_addr: "192.168.1.2".to_string(),
        },
    ]
}

#[tokio::test]
async fn test_normalize_api_url() {
    // If it ends with /, it is returned as is
    assert_eq!(
        normalize_api_url("https://example.com/"),
        "https://example.com/"
    );
    assert_eq!(
        normalize_api_url("https://example.com/somepath/"),
        "https://example.com/somepath/"
    );
    assert_eq!(
        normalize_api_url("https://example.com:8080/"),
        "https://example.com:8080/"
    );
    assert_eq!(
        normalize_api_url("https://example.com:8080/somepath/"),
        "https://example.com:8080/somepath/"
    );
    // If it does not end with /, and has no path, append the default path
    assert_eq!(
        normalize_api_url("https://example.com"),
        format!("https://example.com/{}", consts::UDS_ACTOR_ENDPOINT)
    );
    assert_eq!(
        normalize_api_url("https://example.com:8080"),
        format!("https://example.com:8080/{}", consts::UDS_ACTOR_ENDPOINT)
    );
    // If it does not end with /, but has a path, append also the default path
    assert_eq!(
        normalize_api_url("https://example.com/somepath"),
        format!(
            "https://example.com/somepath/{}",
            consts::UDS_ACTOR_ENDPOINT
        )
    );
}

#[tokio::test]
async fn test_enumerate_authenticators() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api().await;
    let result = types::ApiResponse::<Vec<types::Authenticator>> {
        result: vec![
            types::Authenticator {
                auth_id: "auth1".to_string(),
                auth_small_name: "Auth One".to_string(),
                auth: "auth1".to_string(),
                auth_type: "type1".to_string(),
                priority: 1,
                is_custom: false,
            },
            types::Authenticator {
                auth_id: "auth2".to_string(),
                auth_small_name: "Auth Two".to_string(),
                auth: "auth2".to_string(),
                auth_type: "type2".to_string(),
                priority: 2,
                is_custom: true,
            },
        ],
        error: None,
    };
    let _m = server
        .mock("POST", "/auth/auths")
        .match_header("content-type", "application/json")
        .with_body(serde_json::to_string(&result).unwrap())
        .with_status(200)
        .create_async()
        .await;
    let response = api.enumerate_authenticators().await;
    assert!(
        response.is_ok(),
        "Enumerate authenticators failed: {:?}",
        response
    );
    let auths = response.unwrap();
    assert_eq!(auths.len(), 2);
    assert_eq!(auths[0].auth_id, "auth1");
    assert_eq!(auths[1].auth_id, "auth2");
}

#[tokio::test]
async fn test_register() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api().await;
    let payload = types::RegisterRequest {
        version: consts::VERSION,
        build: consts::BUILD,
        username: "testuser",
        hostname: "testhost",
        ip: "10.0.0.1",
        mac: "00:11:22:33:44:55",
        command: types::RegisterCommandData {
            pre_command: Some("echo pre".to_string()),
            runonce_command: Some("echo runonce".to_string()),
            post_command: Some("echo post".to_string()),
        },
        log_level: types::LogLevel::Debug,
        os: "linux",
    };

    let result = types::ApiResponse::<String> {
        result: "sometoken".to_string(),
        error: None,
    };

    let payload_value: serde_json::Value = serde_json::to_value(&payload).unwrap();
    info!("Payload for register: {}", payload_value);

    let _m = server
        .mock("POST", "/register")
        .match_header("content-type", "application/json")
        .match_body(Matcher::PartialJson(payload_value))
        .with_body(serde_json::to_string(&result).unwrap())
        .with_status(200)
        .create_async()
        .await;

    let response = api
        .register(
            payload.username,
            payload.hostname,
            &crate::operations::NetworkInterface {
                name: "eth0".to_string(),
                mac: payload.mac.to_string(),
                ip_addr: payload.ip.to_string(),
            },
            &payload.command,
            payload.log_level,
            payload.os,
        )
        .await;

    assert!(response.is_ok(), "Register failed: {:?}", response);
    let token = response.unwrap();
    assert_eq!(token, "sometoken");
}

#[tokio::test]
async fn test_initialize() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api().await;
    let result = types::ApiResponse::<types::InitializationResponse> {
        result: types::InitializationResponse {
            master_token: Some("some_master_token".to_string()),
            token: Some("anothertoken".to_string()),
            unique_id: Some("unique_id_123".to_string()),
            os: Some(crate::config::ActorOsConfiguration {
                action: "do_nothing".to_string(),
                name: "linux".to_string(),
                custom: None,
            }),
        },
        error: None,
    };
    let payload = types::InitializationRequest {
        actor_type: crate::config::ActorType::Managed,
        token: api.get_token().unwrap(),
        version: consts::VERSION,
        build: consts::BUILD,
        id: create_test_id().iter().cloned().map(Into::into).collect(),
    };
    let payload_value: serde_json::Value = serde_json::to_value(&payload).unwrap();
    info!("Payload for initialize: {}", payload_value);

    let _m = server
        .mock("POST", "/initialize")
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(payload_value))
        .with_body(serde_json::to_string(&result).unwrap())
        .with_status(200)
        .create_async()
        .await;

    let response = api.initialize(create_test_id().as_slice()).await;
    assert!(response.is_ok(), "Initialize failed: {:?}", response);
}

#[tokio::test]
async fn test_ready() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api().await;
    let result = types::ApiResponse::<types::CertificateInfo> {
        result: types::CertificateInfo {
            key: "key".to_string(),
            certificate: "certificate".to_string(),
            password: "testpass".to_string(),
            ciphers: "TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384".to_string(),
        },
        error: None,
    };
    let payload = types::ReadyRequest {
        token: api.get_token().unwrap(),
        secret: api.get_secret().unwrap(),
        ip: "10.0.0.1",
        port: 1234,
    };
    let payload_value: serde_json::Value = serde_json::to_value(&payload).unwrap();
    info!("Payload for ready: {}", payload_value);
    let _m = server
        .mock("POST", "/ready")
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(payload_value))
        .with_body(serde_json::to_string(&result).unwrap())
        .with_status(200)
        .create_async()
        .await;
    let response = api.ready(payload.ip, payload.port).await;
    assert!(response.is_ok(), "Ready failed: {:?}", response);
}

#[tokio::test]
async fn test_unmanaged_ready() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api().await;
    let result = types::ApiResponse::<types::CertificateInfo> {
        result: types::CertificateInfo {
            key: "key".to_string(),
            certificate: "certificate".to_string(),
            password: "testpass".to_string(),
            ciphers: "TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384".to_string(),
        },
        error: None,
    };
    let payload = types::UnmanagedReadyRequest {
        id: create_test_id().iter().cloned().map(Into::into).collect(),
        token: api.get_token().unwrap(),
        secret: api.get_secret().unwrap(),
        port: 1234,
    }; // Note: unmanaged actors also use the same ready request
    let payload_value: serde_json::Value = serde_json::to_value(&payload).unwrap();
    info!("Payload for unmanaged ready: {}", payload_value);
    let _m = server
        .mock("POST", "/unmanaged")
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(payload_value))
        .with_body(serde_json::to_string(&result).unwrap())
        .with_status(200)
        .create_async()
        .await;
    let response = api
        .unmanaged_ready(
            create_test_id().as_slice(),
            payload.port,
        )
        .await;
    assert!(response.is_ok(), "Unmanaged ready failed: {:?}", response);
}

#[tokio::test]
async fn test_ready_ip_changed() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api().await;
    let result = types::ApiResponse::<types::CertificateInfo> {
        result: types::CertificateInfo {
            key: "key".to_string(),
            certificate: "certificate".to_string(),
            password: "testpass".to_string(),
            ciphers: "TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384".to_string(),
        },
        error: None,
    };
    let payload = types::ReadyRequest {
        token: api.get_token().unwrap(),
        secret: api.get_secret().unwrap(),
        ip: "10.0.0.1",
        port: 1234,
    };
    let payload_value: serde_json::Value = serde_json::to_value(&payload).unwrap();
    info!("Payload for ready: {}", payload_value);
    let _m = server
        .mock("POST", "/ipchange")
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(payload_value))
        .with_body(serde_json::to_string(&result).unwrap())
        .with_status(200)
        .create_async()
        .await;
    let response = api.notify_new_ip(payload.ip, payload.port).await;
    assert!(response.is_ok(), "Ready failed: {:?}", response);
}

#[tokio::test]
async fn test_logout() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api().await;
    let result = types::ApiResponse::<String> {
        result: "ok".to_string(),
        error: None,
    };
    let payload = types::LogoutRequest {
        actor_type: crate::config::ActorType::Managed,
        id: create_test_id().iter().cloned().map(Into::into).collect(),
        token: api.get_token().unwrap(),
        username: "testuser",
        session_type: "session",
        session_id: "session123",
    };
    let payload_value: serde_json::Value = serde_json::to_value(&payload).unwrap();
    info!("Payload for logout: {}", payload_value);
    let _m = server
        .mock("POST", "/logout")
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(payload_value))
        .with_body(serde_json::to_string(&result).unwrap())
        .with_status(200)
        .create_async()
        .await;
    let response = api
        .logout(
            create_test_id().as_slice(),
            payload.username,
            payload.session_type,
            payload.session_id,
        )
        .await;
    assert!(response.is_ok(), "Logout failed: {:?}", response);
}

#[tokio::test]
async fn test_log() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api().await;
    let result = types::ApiResponse::<String> {
        result: "ok".to_string(),
        error: None,
    };
    let payload = types::LogRequest {
        token: api.get_token().unwrap(),
        level: types::LogLevel::Info,
        message: "Test log message",
        timestamp: 1234567890,
    };
    let mut payload_map = serde_json::to_value(&payload)
        .unwrap()
        .as_object()
        .unwrap()
        .clone();
    // Remove timestamp from payload_map, as it is dynamic
    payload_map.remove("timestamp");
    let payload_value = serde_json::Value::Object(payload_map);

    info!("Payload for log: {}", payload_value);
    let _m = server
        .mock("POST", "/log")
        .match_header("content-type", "application/json")
        .match_body(Matcher::PartialJson(payload_value))
        .with_body(serde_json::to_string(&result).unwrap())
        .with_status(200)
        .create_async()
        .await;
    let response = api.log(payload.level, payload.message).await;
    assert!(response.is_ok(), "Log failed: {:?}", response);
}

#[tokio::test]
async fn test_test() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api().await;
    let result = types::ApiResponse::<String> {
        result: "ok".to_string(),
        error: None,
    };
    let payload = types::TestRequest {
        actor_type: crate::config::ActorType::Managed,
        token: api.get_token().unwrap(),
    };
    let payload_value: serde_json::Value = serde_json::to_value(&payload).unwrap();
    info!("Payload for test: {}", payload_value);
    let _m = server
        .mock("POST", "/test")
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(payload_value))
        .with_body(serde_json::to_string(&result).unwrap())
        .with_status(200)
        .create_async()
        .await;
    let response = api.test().await;
    assert!(response.is_ok(), "Test failed: {:?}", response);
}
