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
use crate::platform;

pub async fn task(max_idle: Option<u32>, platform: platform::Platform) -> anyhow::Result<()> {
    let max_idle = std::time::Duration::from_secs(max_idle.unwrap_or(0) as u64);
    if max_idle.as_secs() == 0 {
        // Wait until signaled
        platform.session_manager().wait().await;
        return Ok(());
    }

    let operations = platform.operations();
    let session_manager = platform.session_manager();
    // Initialize idle timer if platform supports it
    operations.init_idle_timer()?;

    let mut notified = false;

    while session_manager.is_running().await {
        // Get current idle time
        let idle = match operations.get_idle_duration() {
            Ok(idle) => idle,
            Err(e) => {
                shared::log::error!("Failed to get idle time: {}", e);
                // If no idle, consider it as zero
                std::time::Duration::from_secs(0)
            }
        };
        // If idle is less than 3 seconds, reset notified flag
        // 3 seconds because we check for idle every second, so if user is active,
        // idle will always be less than 3 seconds (1 second of inactivity + 1 second of check + some margin)
        if idle.as_secs() < 3 {
            notified = false;
        }

        // Notify user:
        // TODO: implement notification using fltk (for portability)
        if idle.as_secs() > 0 && !notified {
            shared::log::info!("User idle for {:?} seconds", idle.as_secs());
            notified = true;
        }
        if idle > max_idle {
            shared::log::info!("Max idle time reached, stopping session");
            break;
        }
        // Wait inside the session_manager for a while (1 second or until signaled)
        session_manager.wait_timeout(std::time::Duration::from_secs(1)).await;
    }
    // Notify session manager to stop session
    platform.session_manager().stop().await;

    Ok(())
}
