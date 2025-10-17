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

pub async fn task(platform: platform::Platform) -> anyhow::Result<Option<String>> {
    log::info!("Signal handler started");
    let session_manager = platform.session_manager();
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let mut sigint = signal(SignalKind::interrupt()).unwrap();

        tokio::select! {
            _ = sigterm.recv() => {
                log::info!("Received SIGTERM");
            },
            _ = sigint.recv() => {
                log::info!("Received SIGINT");
            }
            _ = session_manager.wait() => {
                log::info!("Stop notified");
                return Ok(None);
            }
        }
        // Notify to stop
        session_manager.stop().await;
    
    }

    #[cfg(windows)]
    {
        // On windows, we don't have signals, just wait forever
        // The service control handler will notify us to stop
        session_manager.wait().await;
        log::info!("Stop notified");
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
   // Test on unix that works the signal handler
    #[cfg(unix)]
    #[tokio::test]
    async fn test_signal_handler() {
        log::setup_logging("debug", log::LogType::Tests);
        let (platform, calls) = crate::testing::mock::mock_platform(None, None, None, None).await;
        tokio::spawn(async move {
            let res = super::task(platform).await;
            assert!(res.is_ok());
            assert!(res.unwrap().is_none());
        });

        // Wait a bit to ensure the signal handler is running
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        // Send SIGTERM to ourselves
        let pid = std::process::id();
        unsafe {
            libc::kill(pid as i32, libc::SIGTERM);
        }
        // Wait a bit to ensure the signal is handled
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        
        log::info!("Calls: {:?}", calls.dump());
    }
}