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
use super::api::*;
use super::{consts, types};

use shared::log::{self, info};

use mockito::{Matcher, Server};
use serde_json::json;

// Helper to create a ServerRestApi pointing to mockito server
// Helper to create a mockito server and a ServerRestApi pointing to it
async fn setup_server_and_api() -> (mockito::ServerGuard, ServerRestSession) {
    log::setup_logging("debug", log::LogType::Tests);

    info!("Setting up mock server and API client");
    let server = Server::new_async().await;
    let url = server.url();
    // Pass the base url (without /ui) to the API
    (
        server,
        ServerRestSession::new(&url, false, std::time::Duration::from_secs(5), true).unwrap(),
    )
}

#[tokio::test]
async fn test_initialize() {
    shared::log::setup_logging("debug", shared::log::LogType::Tests);
    let (mut server, api) = setup_server_and_api().await;
    let result = types::ApiResponse::<types::InitializationResult> {
        result: types::InitializationResult {
            master_token: Some("some_master_token".to_string()),
            token: Some("anothertoken".to_string()),
            unique_id: Some("unique_id_123".to_string()),
            os: Some(types::ActorOsConfigurationType {
                action: "do_nothing".to_string(),
                name: "linux".to_string(),
                custom: None,
            }),
        },
        error: None,
    };
    let _m = server
        .mock("POST", "/initialize")
        .match_header("content-type", "application/json")
        .match_body(Matcher::Json(json!({"type":"managed","token":"linux","version":"5.0.0","build":"19452","id":[{"name":"eth0","mac":"00:11:22:33:44:55","ip":"10.0.0.1"},{"name":"wlan0","mac":"66:77:88:99:AA:BB","ip":"10.0.0.2"}]})))
        .with_body(serde_json::to_string(&result).unwrap())
        .with_status(200)
        .create_async()
        .await;
    let response = api
        .initialize(
            "linux",
            vec![
                types::InterfaceInfo {
                    name: "eth0".to_string(),
                    mac: "00:11:22:33:44:55".to_string(),
                    ip: "10.0.0.1".to_string(),
                },
                types::InterfaceInfo {
                    name: "wlan0".to_string(),
                    mac: "66:77:88:99:AA:BB".to_string(),
                    ip: "10.0.0.2".to_string(),
                },
            ]
            .as_slice(),
            Some(consts::MANAGED),
        )
        .await;
    assert!(response.is_ok(), "Initialize failed: {:?}", response);
}
