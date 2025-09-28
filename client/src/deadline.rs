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

pub async fn task(deadline: Option<u32>, platform: platform::Platform) -> anyhow::Result<()> {
    let deadline = std::time::Duration::from_secs(deadline.unwrap_or(0) as u64);
    let deadline = if deadline > std::time::Duration::from_secs(300) {
        deadline - std::time::Duration::from_secs(300)
    } else {
        std::time::Duration::from_secs(0)
    };
    // If no deadline, just wait until signaled
    if deadline.as_secs() == 0 {
        // Wait until signaled
        platform.session_manager().wait().await;
        return Ok(());
    }

    // Deadline timer, simply wait until deadline is reached inside the session_manager
    // But leave a 5 mins to notify before deadline
    if platform.session_manager().wait_timeout(deadline).await {
        shared::log::info!("Deadline notification reached, notifying user");

        // Notify user
        // TODO: Implement notification using fltk (notification must not block)
        // For now, just log it
        shared::log::info!("Notifying user about deadline");

        // Wait 5 minutes more or until signaled
        if platform
            .session_manager()
            .wait_timeout(std::time::Duration::from_secs(300))
            .await
        {
            shared::log::info!("Session still running after deadline, stopping session");
            // Stop session
            platform.session_manager().stop().await;
        }
    }

    Ok(())
}
