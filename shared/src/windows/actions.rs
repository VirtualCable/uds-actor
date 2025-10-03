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
use crate::actions::Actions;
use crate::log;
use async_trait::async_trait;
use std::sync::Arc;

pub struct WindowsActions;

#[async_trait]
impl Actions for WindowsActions {
    async fn screenshot(&self) -> anyhow::Result<Vec<u8>> {
        log::info!("Screenshot requested (stub)");
        // TODO: Take Windows screenshot
        // 1x1 transparent PNG (RGBA)
        // PNG file bytes: 89 50 4E 47 0D 0A 1A 0A ...
        // This is a minimal valid PNG for a 1x1 transparent pixel
        const PNG_1X1_TRANSPARENT: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];
        Ok(PNG_1X1_TRANSPARENT.to_vec())
    }

    async fn run_script(&self, _script: &str) -> anyhow::Result<String> {
        // TODO: Execute script in Windows (maybe lua??). Stubbed for now.
        Ok("".to_string())
    }
}

pub fn new_actions() -> Arc<impl Actions> {
    Arc::new(WindowsActions)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ensure that the notification does not blocks
    #[tokio::test]
    #[ignore] // Ignore in normal test runs as it requires user interaction
    async fn test_notify_user() {
        crate::log::setup_logging("debug", crate::log::LogType::Tests);
        let not_blocked = Arc::new(tokio::sync::Notify::new());
        let actions = new_actions();
        let gui = crate::gui::GuiHandle::new();
        // Spawn on a separate task, ensuring it does not block
        let not_blocked_spawn = not_blocked.clone();
        let started = Arc::new(tokio::sync::Notify::new());
        let started_spawn = started.clone();
        let gui_task = gui.clone();
        tokio::spawn(async move {
            started_spawn.notify_waiters();
            crate::log::info!("Calling notify_user...");
            actions.notify_user("Test notification", gui_task).await.unwrap();
            crate::log::info!("notify_user completed");
            not_blocked_spawn.notify_waiters();
        });

        // Wait a bit,
        started.notified().await;
        // If notify_user blocks, this will timeout
        let res =
            tokio::time::timeout(std::time::Duration::from_secs(2), not_blocked.notified()).await;
        assert!(res.is_err(), "notify_user should not block");
        // Close all dialogs now
        gui.shutdown();
    }
}
