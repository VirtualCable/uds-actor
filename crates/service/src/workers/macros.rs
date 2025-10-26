// Copyright (c) 2025 Virtual Cable S.L.U.
// All rights reserved.
//
// Redistribution and use in source and binary forms, with or without modification,
// are permitted provided that the following conditions are met:
//
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
#[macro_export]
macro_rules! spawn_workers {
    // spawns a single worker sub-macro
    (@spawn_one $server_info:expr, $platform:expr, $name:literal, $func:path) => {{
        log::info!("{} worker created", $name);
        tokio::spawn({
            let s = $server_info.clone();
            let p = $platform.clone();
            async move {
                if let Err(e) = $func(s, p).await {
                    log::error!("{} worker error: {:?}", $name, e);
            }
        }});
    }};

    (
        $server_info:expr,
        $platform:expr,
        [ $( ($name1:literal, $func1:path) ),* $(,)? ],
        [ $( ($name2:literal, $func2:path) ),* $(,)? ],
        [ $( ($name3:literal, $func3:path) ),* $(,)? ]
    ) => {{
        // Common workers
        $(
            spawn_workers!(@spawn_one $server_info, $platform, $name1, $func1);
        )*

        // Actor type specific workers
        {
            tokio::spawn({
                async move {
                let p = $platform.clone();
                let actor_type = p.config().read().await.actor_type.clone();
                if actor_type.is_managed() {
                    $(
                        spawn_workers!(@spawn_one $server_info, $platform, $name2, $func2);
                    )*
                } else {
                    $(
                        spawn_workers!(@spawn_one $server_info, $platform, $name3, $func3);
                    )*
                }
            }});
        }
    }};
}
