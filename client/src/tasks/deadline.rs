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
use shared::log;

pub async fn task(deadline: Option<u32>, platform: platform::Platform) -> anyhow::Result<Option<String>> {
    let deadline = std::time::Duration::from_secs(deadline.unwrap_or(0) as u64);
    let (deadline, remaining) = if deadline > std::time::Duration::from_secs(300) {
        (
            deadline - std::time::Duration::from_secs(300),
            std::time::Duration::from_secs(300),
        )
    } else {
        (deadline, std::time::Duration::from_secs(0)) // If less than 5 mins, just keep it as is
    };
    // If no deadline, just wait until signaled
    if deadline.as_secs() == 0 {
        log::info!("No deadline set, waiting until signaled");
        // Wait until signaled
        platform.session_manager().wait().await;
        return Ok(None);
    }

    // Deadline timer, simply wait until deadline is reached inside the session_manager
    // But leave a 5 mins to notify before deadline
    if !platform.session_manager().wait_timeout(deadline).await {
        log::info!("Deadline notification reached, notifying user");

        platform
            .actions()
            .notify_user("This session will be stopped in 5 minutes.\nPlease save your work.", platform.gui())
            .await
            .ok();

        // Wait remaining minutes more or until signaled
        let _ = platform.session_manager().wait_timeout(remaining).await;
        log::info!("Session still running after deadline, stopping session");
    } else {
        log::info!("Session signaled or closed, stopping deadline task");
    }

    // Notify session manager to stop session
    platform.session_manager().stop().await;

    Ok(Some(format!(
        "deadline of {}s reached",
        deadline.as_secs() + remaining.as_secs()
    )))
}

#[cfg(test)]
mod tests {
    // Tests for deadline task
    use crate::testing::fake::create_platform;

    #[tokio::test]
    async fn test_deadline_task_deadline() {
        shared::log::setup_logging("debug", shared::log::LogType::Tests);
        let (platform, calls) = create_platform(None, None, None, None).await;
        let session_manager = platform.session_manager();

        // Run deadline task in a separate task with a short deadline (10 seconds)
        let res = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            super::task(Some(1), platform),
        )
        .await;
        calls.assert_called("session::wait_timeout(1s)");
        calls.assert_called("session::wait_timeout(0");  // 0ns, 0s, ..
        calls.assert_called("session::stop()");

        session_manager.stop().await; // Ensure session is stopped

        assert!(res.is_ok(), "Deadline task timed out: {:?}", res);
        shared::log::info!("Calls: {:?}", calls.dump());
    }

    #[tokio::test]
    async fn test_deadline_task_no_deadline() {
        shared::log::setup_logging("debug", shared::log::LogType::Tests);
        let (platform, calls) = create_platform(None, None, None, None).await;
        let session_manager = platform.session_manager();
        // Run deadline task in a separate task with no deadline
        let deadline_handle = tokio::spawn(async move {
            let res = super::task(None, platform).await;
            shared::log::info!("Deadline task finished with result: {:?}", res);
        });
        // Wait a bit to ensure deadline task has started
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert!(session_manager.is_running().await);
        // Wait a bit more, to ensure we are inside the wait
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert!(session_manager.is_running().await);

        // Now stop the session
        session_manager.stop().await;
        shared::log::info!("Session stop requested");
        // Wait for deadline task to finish, at most 5 seconds
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), deadline_handle).await;
        assert!(!session_manager.is_running().await);

        shared::log::info!("Calls: {:?}", calls.dump());
    }
}
