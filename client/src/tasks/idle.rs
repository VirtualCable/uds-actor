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

pub async fn task(
    max_idle: Option<u32>,
    platform: platform::Platform,
) -> anyhow::Result<Option<String>> {
    let max_idle = std::time::Duration::from_secs(max_idle.unwrap_or(0) as u64);
    if max_idle.as_secs() == 0 {
        // Wait until signaled
        platform.session_manager().wait().await;
        return Ok(None);
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
            Err(_) => {
                // If no idle, consider it as zero
                std::time::Duration::from_secs(0)
            }
        };

        let remaining = max_idle.saturating_sub(idle);
        if remaining.as_secs() > 120 && notified {
            // If we have more than 2 minutes remaining, reset notified flag
            notified = false;
            log::debug!("User is active again, resetting notified flag");
            // Also, if any dialogs are open, close them
            platform.gui().close_all_windows();
        }

        // Notify user:
        if !notified && remaining.as_secs() <= 120 {
            platform.actions()
                .notify_user("You have been idle for a while. If no action is taken, the session will be stopped.", platform.gui())
                .await
                .ok();
            log::info!("User idle for {:?} seconds", idle.as_secs());
            notified = true;
        }

        // Debug log every 30 seconds
        if idle.as_secs() % 30 == 0 && idle.as_secs() != 0 {
            log::debug!(
                "User idle for {} seconds ({} remaining)",
                idle.as_secs(),
                remaining.as_secs()
            );
        }

        // If we reach max idle, stop session
        if remaining.as_secs() == 0 {
            let message = format!("idle of {}s reached", max_idle.as_secs());
            shared::log::info!("{}", message);
            // Ensure all windows are closed
            platform.gui().close_all_windows();
            // Use logoff in case of idle, should fire stop process
            operations.logoff().ok();
            // Just in case, ensure session manager is notified to stop
            // On RDP session, we may be disconnected and no message is received
            session_manager.stop().await;

            return Ok(Some(message)); // Message to include on logout reason
        }
        // Wait inside the session_manager for a while (1 second or until signaled)
        session_manager
            .wait_timeout(std::time::Duration::from_secs(1))
            .await;
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    // Tests for idle task
    use crate::testing::dummy::create_platform;

    #[tokio::test]
    async fn test_idle_task_idle() {
        shared::log::setup_logging("debug", shared::log::LogType::Tests);

        let (platform, calls) = create_platform(None, None, None, None).await;
        let session_manager = platform.session_manager();

        // Run idle task in a separate task with a short max_idle (10 seconds)
        let res = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            super::task(Some(1), platform),
        )
        .await;
        calls.assert_called("operations::logoff()");
        session_manager.stop().await; // Ensure session is stopped in any case

        assert!(res.is_ok(), "Idle task timed out: {:?}", res);
        calls.assert_called("operations::init_idle_timer()");
        calls.assert_called("operations::get_idle_duration()");
        calls.assert_called("actions::notify_user(\"");
    }

    // Test max_ilde grater than idle (idle is always 600 in our fake)
    #[tokio::test]
    async fn test_idle_task_no_idle_exceeded() {
        shared::log::setup_logging("debug", shared::log::LogType::Tests);

        let (platform, calls) = create_platform(None, None, None, None).await;
        let session_manager = platform.session_manager();

        // Run idle task in a separate task with a short max_idle (5 seconds)
        let res = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            super::task(Some(6000), platform),
        )
        .await;
        // Should timeout, as idle is 600 seconds, and max_idle is 6000 seconds
        assert!(res.is_err(), "Idle task should have timed out: {:?}", res);
        assert!(session_manager.is_running().await);
        calls.assert_not_called("session::stop()");
        calls.assert_called("operations::init_idle_timer()");
        calls.assert_called("operations::get_idle_duration()");
        calls.assert_not_called("actions::notify_user(\"");
    }

    #[tokio::test]
    async fn test_idle_task_no_idle() {
        shared::log::setup_logging("debug", shared::log::LogType::Tests);

        let (platform, calls) = create_platform(None, None, None, None).await;
        let session_manager = platform.session_manager();

        // Run idle task in a separate task with no max_idle
        let idle_handle = tokio::spawn(async move {
            let res = super::task(None, platform).await;
            shared::log::info!("Idle task finished with result: {:?}", res);
        });
        // Wait a bit to ensure idle task has started
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert!(session_manager.is_running().await);
        // Wait a bit more, to ensure we are inside the wait
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        assert!(session_manager.is_running().await);
        // Session should still be running
        calls.assert_not_called("session::stop()");
        calls.assert_called("session::wait()");

        // Now stop the session
        session_manager.stop().await;
        shared::log::info!("Session stop requested");
        // Wait for idle task to finish, at most 5 seconds
        let _ = tokio::time::timeout(std::time::Duration::from_secs(5), idle_handle).await;
        assert!(!session_manager.is_running().await);
    }
}
