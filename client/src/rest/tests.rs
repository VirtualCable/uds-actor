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
use crate::rest::types::{LoginPayload, LogoutPayload};

use super::api::*;
use shared::log::{self, info};

use mockito::{Matcher, Server};
use serde_json::json;

// Helper to create a ClientRestApi pointing to mockito server
// Helper to create a mockito server and a ClientRestApi pointing to it
async fn setup_server_and_api() -> (mockito::ServerGuard, ClientRestApi) {
    log::setup_logging("debug", log::LogType::Tests);

    info!("Setting up mock server and API client");
    let server = Server::new_async().await;
    let url = server.url();
    // Pass the base url (without /ui) to the API
    (server, ClientRestApi::new(&url, false))
}


#[tokio::test]
async fn test_register() {
    let (mut server, mut api) = setup_server_and_api().await;
    let _m = server
        .mock("POST", "/ui/register")
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(json!({"callback_url": "http://callback"})))
        .with_body("\"ok\"")
        .with_status(200)
        .create_async()
        .await;
    let response = api.register("http://callback").await;
    assert!(response.is_ok(), "Register failed: {:?}", response);
}


#[tokio::test]
async fn test_unregister() {
    let (mut server, mut api) = setup_server_and_api().await;
    let _m = server
        .mock("POST", "/ui/unregister")
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(json!({"callback_url": "http://callback"})))
        .with_body("\"ok\"")
        .with_status(200)
        .create_async()
        .await;
    let _ = api.register("http://callback").await;
    assert!(api.unregister().await.is_ok());
}


#[tokio::test]
async fn test_login() {
    let (mut server, mut api) = setup_server_and_api().await;

    let login_payload = LoginPayload {
        username: "user".to_string(),
        session_type: "type".to_string(),
        callback_url: "cb".to_string(),
    };

    let login_resp_str = r#"{"ip": "127.0.0.1", "hostname": "localhost", "deadline": "2025-01-01T00:00:00Z", "max_idle": 300, "session_id": "sessid"}"#;
    let _m = server
        .mock("POST", "/ui/login")
        .match_header("content-type", "application/json")
        .match_body(Matcher::PartialJson(json!(login_payload))) // Partial match to avoid issues with field order
        .with_status(200)
        .with_body(login_resp_str) // Just return session_id as string
        .create_async()
        .await;
    api.set_callback_url("cb");
    api.set_session_id("sessid");
    let res = api.login("user", Some("type")).await;
    assert!(res.is_ok(), "Login failed: {:?}", res);
    let info = res.unwrap();
    assert_eq!(info.session_id, "sessid");
}


#[tokio::test]
async fn test_logout() {
    let (mut server, mut api) = setup_server_and_api().await;
    let logout_payload = LogoutPayload {
        username: "user".to_string(),
        session_type: "type".to_string(),
        callback_url: "cb".to_string(),
        session_id: "sessid".to_string(),
    };

    let _m = server
        .mock("POST", "/ui/logout")
        .match_header("content-type", "application/json")
        .match_body(Matcher::PartialJson(json!(logout_payload)))
        .with_status(200)
        .with_body("\"ok\"")
        .create_async()
        .await;
    api.set_callback_url(&logout_payload.callback_url);
    api.set_session_id(&logout_payload.session_id);
    let res = api.logout("user", Some("type")).await.is_ok();
    assert!(res, "Logout failed: {:?}", res);
}


#[tokio::test]
async fn test_ping() {
    let (mut server, api) = setup_server_and_api().await;
    let _m = server
        .mock("POST", "/ui/ping")
        .match_header("content-type", "application/json")
        .with_status(200)
        .with_body("\"pong\"")
        .create_async()
        .await;
    assert!(api.ping().await.unwrap());
}
