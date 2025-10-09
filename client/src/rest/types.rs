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

use serde::{Deserialize, Serialize};

/// Payload for registration
#[derive(Debug, Clone, Serialize)]
pub struct RegisterRequest {
    pub callback_url: String,
}

/// Payload for unregistration
#[derive(Debug, Clone, Serialize)]
pub struct UnregisterRequest {
    pub callback_url: String,
}

/// Payload for login
#[derive(Debug, Clone, Serialize)]
pub struct LoginRequest {
    pub username: String,
    pub session_type: String,
}

/// Payload for logout
#[derive(Debug, Clone, Serialize)]
pub struct LogoutRequest {
    pub username: String,
    pub session_type: String,
    pub callback_url: String,
    pub session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
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
    pub fn error(&self) -> anyhow::Error {
        anyhow::anyhow!(self.error.clone().unwrap_or_default())
    }

    pub fn result(self) -> anyhow::Result<T> {
        if self.is_error() {
            Err(self.error())
        } else {
            Ok(self.result)
        }
    }
}

/// Login response
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct LoginResponse {
    pub ip: String,
    pub hostname: String,
    pub deadline: Option<u32>, // Stamp, in seconds, when the session will expire
    pub max_idle: Option<u32>, // Max idle time in seconds
    pub session_id: String,
}
