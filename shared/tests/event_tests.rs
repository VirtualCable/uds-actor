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
use std::thread;
use std::time::Duration;

use shared::windows::WindowsEvent;

#[cfg(windows)]
#[test]
fn event_signal_and_wait() {
    let ev = WindowsEvent::new();
    assert!(!ev.is_set());

    let ev2 = ev.clone();
    let handle = thread::spawn(move || {
        ev2.wait();
        42
    });

    // Signal after a small delay
    thread::sleep(Duration::from_millis(100));
    ev.signal();

    let result = handle.join().unwrap();
    assert_eq!(result, 42);
    assert!(ev.is_set());
}

#[cfg(windows)]
#[test]
fn event_wait_timeout() {
    let ev = WindowsEvent::new();

    // Not signaled, should timeout
    let signaled = ev.wait_timeout(Duration::from_millis(100));
    assert!(signaled.is_err());

    // Now we signal it and it should wake up
    ev.signal();
    let signaled = ev.wait_timeout(Duration::from_millis(100));
    assert!(signaled.is_ok());
}

#[cfg(windows)]
#[test]
fn event_reset() {
    let ev = WindowsEvent::new();
    ev.signal();
    assert!(ev.is_set());

    ev.reset();
    assert!(!ev.is_set());
}

#[cfg(windows)]
#[tokio::test]
async fn event_signal_and_wait_async() {
    let ev = WindowsEvent::new();
    assert!(!ev.is_set());

    let ev2 = ev.clone();
    let handle = tokio::spawn(async move {
        ev2.wait_async().await;
        42
    });

    // Signal after a small delay
    tokio::time::sleep(Duration::from_millis(100)).await;
    ev.signal();

    let result = handle.await.unwrap();
    assert_eq!(result, 42);
    assert!(ev.is_set());
}

#[cfg(windows)]
#[tokio::test]
async fn event_wait_timeout_async() {
    let ev = WindowsEvent::new();

    // Not signaled, should timeout
    let signaled = ev.wait_timeout_async(Duration::from_millis(100)).await.is_ok();
    assert!(!signaled);

    // Now we signal it and it should wake up
    ev.signal();
    let signaled = ev.wait_timeout_async(Duration::from_millis(100)).await.is_ok();
    assert!(signaled);
}
