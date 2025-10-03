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
use async_trait::async_trait;

// Common actions trait for different platforms
#[async_trait]
pub trait Actions: Send + Sync {
    async fn screenshot(&self) -> anyhow::Result<Vec<u8>>;
    async fn run_script(&self, script: &str) -> anyhow::Result<String>;

    // Default implementation for notifying the user: closes dialogs and shows
    // a notification dialog. Implementations may override if platform-specific
    // behavior is required.
    async fn notify_user(&self, message: &str, gui: crate::gui::GuiHandle) -> anyhow::Result<()> {
        crate::log::info!("Notify user: {}", message);
        let message = message.to_string();
        // ensure_dialogs_closed().await;
        // Execute the dialog on a background thread
        tokio::spawn(async move {
            _ = gui.message_dialog("Notification", &message).await;
        });
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub use crate::windows::actions::new_actions;

#[cfg(unix)]
pub use crate::unix::actions::new_actions;
