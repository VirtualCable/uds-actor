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

use crate::{
    config::{ActorConfiguration, ActorOsAction, ActorOsConfiguration, ActorType},
    log::{self, info},
    tls::CertificateInfo,
};

use mockito::{Matcher, Server};

// Helper to create a ServerRestApi pointing to mockito server
// Helper to create a mockito server and a ServerRestApi pointing to it
async fn setup_server_and_api(
    actor_type: Option<ActorType>,
) -> (mockito::ServerGuard, UdsBrokerApi) {
    log::setup_logging("debug", log::LogType::Tests);

    let server = Server::new_async().await;
    let url = server.url() + "/"; // For testing, our base URL will be the mockito server

    let config = ActorConfiguration {
        broker_url: url,
        master_token: Some("token".to_string()),
        actor_type: actor_type.unwrap_or(ActorType::Managed),
        ..Default::default()
    };

    info!("Setting up mock server and API client");
    let broker = UdsBrokerApi::new(config, false, None);
    // Pass the base url (without /ui) to the API
    (server, broker)
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

fn rest_actor_path(method: &str) -> String {
    format!("/{}{}", consts::REST_ACTOR_PATH, method)
}

#[tokio::test]
async fn test_enumerate_authenticators() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api(None).await;
    let result = vec![
        types::Authenticator {
            id: "auth1".to_string(),
            label: "Auth One".to_string(),
            name: "auth1".to_string(),
            auth_type: "type1".to_string(),
            priority: 1,
            custom: false,
        },
        types::Authenticator {
            id: "auth2".to_string(),
            label: "Auth Two".to_string(),
            name: "auth2".to_string(),
            auth_type: "type2".to_string(),
            priority: 2,
            custom: true,
        },
    ];
    let _m = server
        .mock("GET", "/auth/auths")
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
    assert_eq!(auths[0].id, "auth1");
    assert_eq!(auths[1].id, "auth2");
}

#[tokio::test]
async fn test_api_login() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api(None).await;
    let login_req = types::ApiLoginRequest {
        auth: "auth1",
        username: "testuser",
        password: "testpass",
    };
    let result = types::ApiLoginResponse {
        token: "logintoken".to_string(),
        result: "ok".to_string(),
        error: None,
    };
    let payload_value: serde_json::Value = serde_json::to_value(&login_req).unwrap();
    info!("Payload for api_login: {}", payload_value);
    let _m = server
        .mock("POST", "/auth/login")
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(payload_value))
        .with_body(serde_json::to_string(&result).unwrap())
        .with_status(200)
        .create_async()
        .await;
    let response = api
        .api_login(login_req.auth, login_req.username, login_req.password)
        .await;
    assert!(response.is_ok(), "API login failed: {:?}", response);
    let token = response.unwrap();
    assert_eq!(token, "logintoken");
}

#[tokio::test]
async fn test_register() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api(None).await;
    let reg_req = types::RegisterRequest {
        version: crate::consts::VERSION,
        build: crate::consts::BUILD,
        username: "testuser",
        hostname: "testhost",
        ip: "10.0.0.1",
        mac: "00:11:22:33:44:55",
        commands: types::RegisterCommands {
            pre_command: Some("echo pre".to_string()),
            runonce_command: Some("echo runonce".to_string()),
            post_command: Some("echo post".to_string()),
        },
        log_level: types::LogLevel::Debug.into(), // log level as u32
        os: "linux",
    };

    let result = types::ApiResponse::<String> {
        result: "sometoken".to_string(),
        error: None,
    };

    let payload_value: serde_json::Value = serde_json::to_value(&reg_req).unwrap();
    info!("Payload for register: {}", payload_value);

    let _m = server
        .mock("POST", rest_actor_path("register").as_str())
        .match_header("content-type", "application/json")
        .match_body(Matcher::PartialJson(payload_value))
        .with_body(serde_json::to_string(&result).unwrap())
        .with_status(200)
        .create_async()
        .await;

    let response = api.register(&reg_req).await;

    assert!(response.is_ok(), "Register failed: {:?}", response);
    let token = response.unwrap();
    assert_eq!(token, "sometoken");
}

#[tokio::test]
async fn test_initialize() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api(None).await;
    let result = types::ApiResponse::<types::InitializationResponse> {
        result: types::InitializationResponse {
            master_token: Some("some_master_token".to_string()),
            token: Some("anothertoken".to_string()),
            unique_id: Some("unique_id_123".to_string()),
            os: Some(ActorOsConfiguration {
                action: ActorOsAction::None,
                name: "linux".to_string(),
                custom: None,
            }),
        },
        error: None,
    };
    let payload = types::InitializationRequest {
        actor_type: ActorType::Managed,
        token: &api.get_token().unwrap(),
        version: crate::consts::VERSION,
        build: crate::consts::BUILD,
        id: create_test_id().iter().cloned().map(Into::into).collect(),
    };
    let payload_value: serde_json::Value = serde_json::to_value(&payload).unwrap();
    info!("Payload for initialize: {}", payload_value);

    let _m = server
        .mock("POST", rest_actor_path("initialize").as_str())
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
    let (mut server, api) = setup_server_and_api(None).await;
    let result = types::ApiResponse::<CertificateInfo> {
        result: CertificateInfo {
            key: "key".to_string(),
            certificate: "certificate".to_string(),
            password: Some("testpass".to_string()),
            ciphers: Some("TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384".to_string()),
        },
        error: None,
    };
    let payload = types::ReadyRequest {
        token: &api.get_token().unwrap(),
        secret: api.get_secret().unwrap(),
        ip: "10.0.0.1",
        port: 1234,
    };
    let payload_value: serde_json::Value = serde_json::to_value(&payload).unwrap();
    info!("Payload for ready: {}", payload_value);
    let _m = server
        .mock("POST", rest_actor_path("ready").as_str())
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
    let (mut server, api) = setup_server_and_api(Some(ActorType::Unmanaged)).await;
    let result = types::ApiResponse::<CertificateInfo> {
        result: CertificateInfo {
            key: "key".to_string(),
            certificate: "certificate".to_string(),
            password: Some("testpass".to_string()),
            ciphers: Some("TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384".to_string()),
        },
        error: None,
    };
    let payload = types::UnmanagedReadyRequest {
        id: create_test_id().iter().cloned().map(Into::into).collect(),
        token: &api.get_token().unwrap(),
        secret: api.get_secret().unwrap(),
        port: 1234,
    }; // Note: unmanaged actors also use the same ready request
    let payload_value: serde_json::Value = serde_json::to_value(&payload).unwrap();
    info!("Payload for unmanaged ready: {}", payload_value);
    let _m = server
        .mock("POST", rest_actor_path("unmanaged").as_str())
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(payload_value))
        .with_body(serde_json::to_string(&result).unwrap())
        .with_status(200)
        .create_async()
        .await;
    let response = api
        .unmanaged_ready(create_test_id().as_slice(), payload.port)
        .await;
    assert!(response.is_ok(), "Unmanaged ready failed: {:?}", response);
}

#[tokio::test]
async fn test_ready_ip_changed() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api(None).await;
    let result = types::ApiResponse::<CertificateInfo> {
        result: CertificateInfo {
            key: "key".to_string(),
            certificate: "certificate".to_string(),
            password: Some("testpass".to_string()),
            ciphers: Some("TLS_AES_128_GCM_SHA256:TLS_AES_256_GCM_SHA384".to_string()),
        },
        error: None,
    };
    let payload = types::ReadyRequest {
        token: &api.get_token().unwrap(),
        secret: api.get_secret().unwrap(),
        ip: "10.0.0.1",
        port: 1234,
    };
    let payload_value: serde_json::Value = serde_json::to_value(&payload).unwrap();
    info!("Payload for ready: {}", payload_value);
    let _m = server
        .mock("POST", rest_actor_path("ipchange").as_str())
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
    let (mut server, api) = setup_server_and_api(None).await;
    let result = types::ApiResponse::<String> {
        result: "ok".to_string(),
        error: None,
    };
    let payload = types::LogoutRequest {
        actor_type: crate::config::ActorType::Managed,
        id: create_test_id().iter().cloned().map(Into::into).collect(),
        token: &api.get_token().unwrap(),
        username: "testuser",
        session_type: "session",
        session_id: "session123",
    };
    let payload_value: serde_json::Value = serde_json::to_value(&payload).unwrap();
    info!("Payload for logout: {}", payload_value);
    let _m = server
        .mock("POST", rest_actor_path("logout").as_str())
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
    let (mut server, api) = setup_server_and_api(None).await;
    let result = types::ApiResponse::<String> {
        result: "ok".to_string(),
        error: None,
    };
    let payload = types::LogRequest {
        token: &api.get_token().unwrap(),
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
        .mock("POST", rest_actor_path("log").as_str())
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
async fn test_test_managed() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api(None).await;
    let result = types::ApiResponse::<String> {
        result: "ok".to_string(),
        error: None,
    };
    let payload = types::TestRequest {
        actor_type: crate::config::ActorType::Managed,
        token: &api.get_token().unwrap(),
    };
    let payload_value: serde_json::Value = serde_json::to_value(&payload).unwrap();
    info!("Payload for test: {}", payload_value);
    let _m = server
        .mock("POST", rest_actor_path("test").as_str())
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(payload_value))
        .with_body(serde_json::to_string(&result).unwrap())
        .with_status(200)
        .create_async()
        .await;
    let response = api.test().await;
    assert!(response.is_ok(), "Test failed: {:?}", response);
}

#[tokio::test]
async fn test_test_unmanaged() {
    log::setup_logging("debug", log::LogType::Tests);
    let (mut server, api) = setup_server_and_api(Some(ActorType::Unmanaged)).await;

    let result = types::ApiResponse::<String> {
        result: "ok".to_string(),
        error: None,
    };
    let payload = types::TestRequest {
        actor_type: crate::config::ActorType::Unmanaged,
        token: &api.get_token().unwrap(),
    };
    let payload_value: serde_json::Value = serde_json::to_value(&payload).unwrap();
    info!("Payload for test unmanaged: {}", payload_value);
    let _m = server
        .mock("POST", rest_actor_path("test").as_str())
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(payload_value))
        .with_body(serde_json::to_string(&result).unwrap())
        .with_status(200)
        .create_async()
        .await;
    let response = api.test().await;
    assert!(response.is_ok(), "Test unmanaged failed: {:?}", response);
}
