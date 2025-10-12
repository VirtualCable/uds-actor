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
use crate::windows::safehandle::SafeHandle;
use std::time::Duration;
use windows::Win32::Foundation::{WAIT_OBJECT_0, WAIT_TIMEOUT};
use windows::Win32::System::Threading::{
    CreateEventW, INFINITE, ResetEvent, SetEvent, WaitForSingleObject,
};

#[derive(Clone, Debug)]
pub struct WindowsEvent {
    handle: SafeHandle,
}

#[allow(dead_code)]
impl WindowsEvent {
    pub fn new() -> Self {
        unsafe {
            // Manual reset event, initial state non-signaled
            let handle = CreateEventW(
                None,  // default security
                true,  // manual reset
                false, // initial state: not signaled
                None,  // no name
            )
            .expect("Failed to create event");
            WindowsEvent {
                handle: SafeHandle::new(handle),
            }
        }
    }
    pub fn is_valid(&self) -> bool {
        self.handle.is_valid()
    }

    pub fn into_raw(self) -> *mut core::ffi::c_void {
        // Consumes the Event and returns the SafeHandle, without closing it
        self.handle.into_raw()
    }

    pub fn from_raw(handle: *mut core::ffi::c_void) -> Self {
        let handle = SafeHandle::from_raw(handle);
        WindowsEvent { handle }
    }

    pub fn get(&self) -> SafeHandle {
        // Returns a clone of the SafeHandle
        self.handle.clone()
    }
}

impl crate::sync::traits::EventLike for WindowsEvent {
    /// Blocks until the event is signaled
    fn wait(&self) {
        unsafe {
            let res = WaitForSingleObject(self.handle.get(), INFINITE);
            assert!(res == WAIT_OBJECT_0, "WaitForSingleObject failed");
        }
    }

    /// Blocks until the event is signaled or the timeout expires
    /// Returns true if the event was signaled, false if the timeout expired
    fn wait_timeout(&self, timeout: Duration) -> bool {
        unsafe {
            let ms = timeout.as_millis().min(u32::MAX as u128) as u32;
            let res = WaitForSingleObject(self.handle.get(), ms);
            match res {
                x if x == WAIT_OBJECT_0 => true,
                x if x == WAIT_TIMEOUT => false,
                _ => panic!("WaitForSingleObject failed: {res:?}"),
            }
        }
    }

    /// Signals the event (wakes up all waiters)
    fn signal(&self) {
        unsafe {
            let ok = SetEvent(self.handle.get()).is_ok();
            debug_assert!(ok, "SetEvent failed");
        }
    }

    /// Resets the event to non-signaled state (optional)
    fn reset(&self) {
        unsafe {
            let ok = ResetEvent(self.handle.get()).is_ok();
            debug_assert!(ok, "ResetEvent failed");
        }
    }

    /// If is set to true, the event is in a signaled state
    fn is_set(&self) -> bool {
        unsafe {
            let res = WaitForSingleObject(self.handle.get(), 0);
            res == WAIT_OBJECT_0
        }
    }
}

impl Default for WindowsEvent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::traits::EventLike;
    use std::thread;
    use std::time::{Duration, Instant};

    #[test]
    fn event_wait_blocks_until_signal() {
        let event = WindowsEvent::new();
        let event_clone = event.clone();

        let handle = thread::spawn(move || {
            // Wait 100ms and then signal the event
            thread::sleep(Duration::from_millis(100));
            event_clone.signal();
        });

        let start = Instant::now();
        event.wait();
        let elapsed = start.elapsed();

        // Should have waited at least 100ms
        assert!(elapsed >= Duration::from_millis(100));
        handle.join().unwrap();
    }

    #[test]
    fn event_signal_wakes_all_waiters() {
        let event = WindowsEvent::new();
        let mut handles = vec![];

        for _ in 0..5 {
            let event_clone = event.clone();
            handles.push(thread::spawn(move || {
                event_clone.wait();
            }));
        }

        // Signal the event and wait for all threads to finish
        thread::sleep(Duration::from_millis(50));
        event.signal();
        for h in handles {
            h.join().unwrap();
        }
    }

    #[test]
    fn event_reset_blocks_again() {
        let event = WindowsEvent::new();
        event.signal();
        event.wait(); // Should not block

        event.reset();

        let event_clone = event.clone();
        let handle = thread::spawn(move || {
            // Wait 50ms and then signal the event
            thread::sleep(Duration::from_millis(50));
            event_clone.signal();
        });

        let start = Instant::now();
        event.wait();
        let elapsed = start.elapsed();
        assert!(elapsed >= Duration::from_millis(50));
        handle.join().unwrap();
    }

    #[test]
    fn event_wait_timeout() {
        let event = WindowsEvent::new();
        let start = Instant::now();
        let result = event.wait_timeout(Duration::from_millis(100));
        let elapsed = start.elapsed();

        // Should return false and wait for less than 100ms
        assert!(!result);
        assert!(elapsed < Duration::from_millis(200));

        // Now signal the event and check that it returns true
        event.signal();
        let result = event.wait_timeout(Duration::from_millis(100));
        assert!(result);
    }

    #[test]
    fn event_is_set() {
        let event = WindowsEvent::new();
        assert!(!event.is_set());

        event.signal();
        assert!(event.is_set());

        event.reset();
        assert!(!event.is_set());
    }
}
