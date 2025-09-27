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
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[allow(dead_code)]
pub fn run_with_timeout<F, R>(timeout: Duration, func: F) -> Result<R, &'static str>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let (tx, rx) = mpsc::channel();

    // Main thread to run the function
    let tx_clone = tx.clone();
    thread::spawn(move || {
        let result = func();
        let _ = tx_clone.send(Some(result));
    });

    // timeout thread
    thread::spawn(move || {
        thread::sleep(timeout);
    let _ = tx.send(None); // if the timeout wins, send None
    });

    match rx.recv() {
        Ok(Some(result)) => Ok(result),
        Ok(None) => Err("Timeout reached"),
        Err(_) => Err("Error receiving result"),
    }
}
