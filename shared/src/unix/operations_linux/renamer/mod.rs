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
use std::collections::HashMap;
use std::sync::OnceLock;

use anyhow::Result;

mod alt;
mod debian;
mod opensuse;
mod redhat;

static RENAMERS: OnceLock<HashMap<&'static str, fn(&str) -> Result<()>>> = OnceLock::new();

pub(super) fn renamer(new_name: &str, os_name: &str) -> Result<()> {
    let renamer = RENAMERS.get_or_init(|| {
        let mut m = HashMap::new();
        for (fnc, known_names) in &[
            (alt::rename as fn(&str) -> Result<()>, alt::KNOWN_NAMES),
            (
                debian::rename as fn(&str) -> Result<()>,
                debian::KNOWN_NAMES,
            ),
            (
                opensuse::rename as fn(&str) -> Result<()>,
                opensuse::KNOWN_NAMES,
            ),
            (
                redhat::rename as fn(&str) -> Result<()>,
                redhat::KNOWN_NAMES,
            ),
        ] {
            for &name in *known_names {
                m.insert(name, *fnc);
            }
        }
        m
    });

    // Search for a renamer
    for (key, func) in renamer.iter() {
        if os_name.contains(key) {
            return func(new_name);
        }
    }

    // Use debian renamer as fallback
    debian::rename(new_name)
}
