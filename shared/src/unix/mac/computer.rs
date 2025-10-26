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
use std::process::Command;

use anyhow::Result;

use crate::log;

// Get computer name on macos
pub(super) fn get_computer_name() -> Result<String> {
    let output = Command::new("scutil")
        .arg("--get")
        .arg("ComputerName")
        .output()?;

    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(name)
    } else {
        Err(anyhow::anyhow!(
            "Failed to get computer name: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

pub(super) fn join_domain(_options: &crate::operations::JoinDomainOptions) -> Result<()> {
    // Currently, no join domain implementation for macOS
    log::warn!("join_domain is not implemented for macOS");
    Ok(())
}

// For macos
/// Ensures that the system time is updated by restarting systemd-timesyncd if it is active.
pub(super) fn refresh_system_time() -> Result<()> {
    let output = Command::new("/usr/sbin/sntp")
        .arg("-sS")
        .arg("time.apple.com")
        .output()?;

    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_computer_name() {
        crate::log::setup_logging("debug", crate::log::LogType::Tests);
        let res = get_computer_name();
        assert!(res.is_ok());
        let name = res.unwrap();
        println!("Computer name: {}", name);
        assert!(!name.is_empty());
    }
}