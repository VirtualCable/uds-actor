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
use std::fs::{self, File};
use std::io::{self, BufRead, Write};
use std::process::Command;

use anyhow::Result;

pub(super) const KNOWN_NAMES: &[&str] = &["opensuse", "suse", "sles", "opensuse-leap"];

pub(super) fn rename(new_name: &str) -> Result<()> {
    crate::log::debug!("using SUSE renamer");

    fs::write("/etc/hostname", new_name)?;

    let _ = Command::new("hostnamectl")
        .arg("set-hostname")
        .arg(new_name)
        .status()?;
    let _ = Command::new("/bin/hostname").arg(new_name).status()?;

    if let Ok(file) = File::open("/etc/hosts") {
        let lines: Vec<String> = io::BufReader::new(file)
            .lines()
            .filter_map(Result::ok)
            .collect();

        let mut hosts = File::create("/etc/hosts")?;
        writeln!(hosts, "127.0.1.1\t{}", new_name)?;
        for l in lines {
            if !l.starts_with("127.0.1.1") {
                writeln!(hosts, "{}", l)?;
            }
        }
    }

    Ok(())
}
