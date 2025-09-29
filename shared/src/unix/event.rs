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
use crate::sync::traits::EventLike;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct UnixEvent(Arc<(Mutex<bool>, Condvar)>);

impl UnixEvent {
    pub fn new() -> Self {
        Self(Arc::new((Mutex::new(false), Condvar::new())))
    }
}

impl EventLike for UnixEvent {
    fn wait(&self) {
        let (lock, cvar) = &*self.0;
        let mut set = lock.lock().unwrap();
        while !*set {
            set = cvar.wait(set).unwrap();
        }
    }

    fn wait_timeout(&self, timeout: Duration) -> bool {
        let (lock, cvar) = &*self.0;
        let set = lock.lock().unwrap();
        let (set, result) = cvar.wait_timeout_while(set, timeout, |s| !*s).unwrap();
        *set || !result.timed_out()
    }

    fn signal(&self) {
        let (lock, cvar) = &*self.0;
        let mut set = lock.lock().unwrap();
        *set = true;
        cvar.notify_all();
    }

    fn reset(&self) {
        let (lock, _) = &*self.0;
        *lock.lock().unwrap() = false;
    }

    fn is_set(&self) -> bool {
        let (lock, _) = &*self.0;
        *lock.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_wait_blocks_until_signal() {
        let event = UnixEvent::new();
        let event_clone = event.clone();

        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            event_clone.signal();
        });

        event.wait();
    }
    #[test]
    fn event_signal_wakes_all_waiters() {
        let event = UnixEvent::new();
        let mut handles = vec![];

        for _ in 0..5 {
            let event_clone = event.clone();
            handles.push(std::thread::spawn(move || {
                event_clone.wait();
            }));
        }

        std::thread::sleep(std::time::Duration::from_millis(50));
        event.signal();
        for h in handles {
            h.join().unwrap();
        }
    }
    #[test]
    fn event_reset_blocks_again() {
        let event = UnixEvent::new();
        event.signal();
        event.wait(); // Should not block

        event.reset();

        let event_clone = event.clone();
        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            event_clone.signal();
        });

        let start = std::time::Instant::now();
        event.wait();
        let elapsed = start.elapsed();
        assert!(elapsed >= std::time::Duration::from_millis(50));
        handle.join().unwrap();
    }

    #[test]
    fn test_event_wait_timeout() {
        let event = UnixEvent::new();

        // Test that wait_timeout returns false when timeout occurs
        let start = std::time::Instant::now();
        let result = event.wait_timeout(std::time::Duration::from_millis(100));
        let elapsed = start.elapsed();
        assert!(!result);
        assert!(elapsed >= std::time::Duration::from_millis(100));

        // Now signal the event and test that wait_timeout returns true
        event.signal();
        let result = event.wait_timeout(std::time::Duration::from_millis(100));
        assert!(result);
    }

    #[test]
    fn test_event_is_set() {
        let event = UnixEvent::new();
        assert!(!event.is_set());
        event.signal();
        assert!(event.is_set());
        event.reset();
        assert!(!event.is_set());
    }
}
