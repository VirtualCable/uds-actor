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
use crate::sync::event::{Event, EventLike};
use std::sync::{
    Arc, LazyLock,
    atomic::{AtomicIsize, Ordering},
};
use windows::{
    Win32::{Foundation::*, System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*},
    core::PCWSTR,
};

use crate::log;

static CLASS_NAME: &str = "rds_launcher_app";
static CLASS_NAME_WIDE: LazyLock<widestring::U16CString> =
    LazyLock::new(|| widestring::U16CString::from_str_truncate(CLASS_NAME));

fn get_class_name() -> PCWSTR {
    PCWSTR(CLASS_NAME_WIDE.as_ptr())
}

#[allow(dead_code)]
pub struct MsgWindow {
    pub stop_notify: Event,
}

#[allow(dead_code)]
impl MsgWindow {
    pub fn new(stop_notify: Event) -> Self {
        Self { stop_notify }
    }

    fn create_invisible_window(event: Event) -> HWND {
        let h_instance = unsafe {
            GetModuleHandleW(None)
                .unwrap_or_else(|e| panic!("Failed to get module handle: {:?}", e))
        }
        .into();

        let class_name = get_class_name();

        let wc = WNDCLASSW {
            lpfnWndProc: Some(launcher_window_proc),
            hInstance: h_instance,
            lpszClassName: class_name,
            style: CS_HREDRAW | CS_VREDRAW,
            ..Default::default()
        };

        unsafe {
            RegisterClassW(&wc);

            let event_ptr = event.into_raw();
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(0),
                class_name,
                PCWSTR(widestring::U16CString::from_str_truncate("RDS Invisible Window").as_ptr()),
                WS_POPUP | WS_DISABLED, // 👈 No borders, no interaction, 100% hidden
                0,
                0,
                0,
                0,
                None, // 👈 Not using HWND_MESSAGE
                None,
                Some(h_instance),
                None, // No lpParam
            )
            .unwrap_or_else(|e| {
                log::error!("Error creating invisible window: {:?}", e);
                panic!("Error creating invisible window!!!: {:?}", e);
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, event_ptr as isize);
            hwnd
        }
    }

    fn process_message(msg: &MSG) {
        unsafe {
            let _ = TranslateMessage(msg);
            DispatchMessageW(msg);
        }
    }

    fn destroy_window(hwnd: &HWND) {
        if !hwnd.0.is_null() {
            unsafe {
                let _ = DestroyWindow(*hwnd);
                let _ = UnregisterClassW(get_class_name(), None);
            }
            log::debug!("🧹 Window destroyed");
        }
    }

    fn process_messages(hwnd: &HWND) {
        let mut msg = MSG::default();
        log::debug!("🕒 Starting message loop for HWND: {:?}", hwnd);

        while unsafe { GetMessageW(&mut msg, None, 0, 0).into() } {
            log::debug!("📨 Got message: {} for HWND: {:?}", msg.message, msg.hwnd);

            MsgWindow::process_message(&msg);
        }

        log::debug!("🏁 Message loop ended");
    }

    fn do_close(hwnd: &HWND) {
        if !hwnd.0.is_null() {
            log::debug!("🛑 Closing messages window");
            unsafe {
                let _ = PostMessageW(Some(*hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
    }

    // Execute this on main thread
    pub fn task(&mut self) {
        log::debug!("🛠️ Starting message window task");
        let hwnd_shared = Arc::new(AtomicIsize::new(0));
        let hwnd_for_msgs = hwnd_shared.clone();

        // Thread for message loop

        // Thread to watch for stop notification and post quit message
        let stop_notify = self.stop_notify.clone();
        let hwnd_for_waiter = hwnd_shared.clone();
        let waiter_thread = std::thread::spawn(move || {
            log::debug!("🕒 Waiting for stop notification");
            stop_notify.wait();
            log::debug!("🛑 Stop notification received, posting quit message");
            // Do load after stop notification to ensure the window is created
            let hwnd_val = hwnd_for_waiter.load(Ordering::SeqCst);
            if hwnd_val != 0 {
                let hwnd = HWND(hwnd_val as _);
                MsgWindow::do_close(&hwnd);
            }
        });

        let stop_notify = self.stop_notify.clone();
        log::debug!("🛠️ Creating invisible window for message processing on its own thread");
        let hwnd_val = Self::create_invisible_window(stop_notify).0 as isize;
        hwnd_for_msgs.store(hwnd_val, Ordering::SeqCst);
        log::debug!("🕒 Waiting for messages");
        let hwnd = HWND(hwnd_val as _);
        Self::process_messages(&hwnd);
        hwnd_for_msgs.store(0, Ordering::SeqCst); // Clear the hwnd after processing

        // If we stop, everyone should do it now also :)
        log::debug!("🛑 Stop notification received, signaling stop");
        self.stop_notify.signal();
        // Wait for both threads to finish
        let _ = waiter_thread.join();

        // Sleep for a while to ensure the message processing is complete
        std::thread::sleep(std::time::Duration::from_millis(100));

        log::debug!("🧹 Destroying invisible window after message processing");
        Self::destroy_window(&hwnd);
        log::info!("🛠️ Message window task completed");
    }
}

extern "system" fn launcher_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let event = unsafe {
        let event_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
        Event::from_raw(event_ptr as _)
    };

    unsafe {
        match msg {
            WM_CLOSE => {
                log::debug!("🔔 Received message: WM_CLOSE {}", msg);
                if event.is_valid() {
                    event.signal();
                }
                PostQuitMessage(0);
                LRESULT(0)
            }
            WM_ENDSESSION => {
                log::debug!("🔔 Received message: WM_ENDSESSION {}", msg);
                // Sleep a while to ensure the message is processed
                if event.is_valid() {
                    event.signal();
                }

                // Before posting quit message, we can do some cleanup if needed
                log::debug!("Waiting for cleanup before quitting");
                std::thread::sleep(std::time::Duration::from_millis(1500));
                log::debug!("🛑 Ending session, posting quit message");
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::test_utils::run_with_timeout;

    #[test]
    fn test_msg_window_creation() {
        log::setup_logging("debug", log::LogType::Tests);

        let stop_notify = Event::new();
        let msg_window = MsgWindow::new(stop_notify);
        assert!(!msg_window.stop_notify.is_set());
    }

    #[test]
    fn test_msg_create_invisible_window() {
        log::setup_logging("debug", log::LogType::Tests);

        let hwnd = MsgWindow::create_invisible_window(Event::new());
        assert!(
            !hwnd.0.is_null(),
            "Invisible window should be created successfully"
        );
        MsgWindow::destroy_window(&hwnd);
    }

    #[test]
    fn test_msg_window_proc() {
        log::setup_logging("debug", log::LogType::Tests);
        let event = Event::new();

        let hwnd = MsgWindow::create_invisible_window(event.clone());
        let msg = WM_CLOSE;
        let result = launcher_window_proc(hwnd, msg, WPARAM(0), LPARAM(0));
        assert_eq!(result.0, 0, "WM_CLOSE should return 0");
        let result = launcher_window_proc(hwnd, WM_ENDSESSION, WPARAM(0), LPARAM(0));
        assert_eq!(result.0, 0, "WM_ENDSESSION should return 0");

        MsgWindow::destroy_window(&hwnd);
    }

    #[test]
    fn test_msg_process_wm_close() {
        log::setup_logging("debug", log::LogType::Tests);

        run_with_timeout(Duration::from_secs(4), move || {
            let hwnd = MsgWindow::create_invisible_window(Event::new());
            unsafe {
                PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)).unwrap_or_else(|e| {
                    panic!("Failed to post message: {:?}", e);
                });
            }
            MsgWindow::process_messages(&hwnd);
            MsgWindow::destroy_window(&hwnd);
        })
        .expect("msg_thread should run within timeout");
    }

    #[test]
    fn test_msg_process_wm_end_session() {
        log::setup_logging("debug", log::LogType::Tests);

        run_with_timeout(Duration::from_secs(4), move || {
            let hwnd = MsgWindow::create_invisible_window(Event::new());
            unsafe {
                PostMessageW(Some(hwnd), WM_ENDSESSION, WPARAM(0), LPARAM(0)).unwrap_or_else(|e| {
                    panic!("Failed to post message: {:?}", e);
                });
            }
            MsgWindow::process_messages(&hwnd);
            MsgWindow::destroy_window(&hwnd);
        })
        .expect("msg_thread should run within timeout");
    }

    #[test]
    fn test_message_window_starts_and_stops() {
        run_with_timeout(Duration::from_secs(5), move || {
            log::setup_logging("debug", log::LogType::Tests);

            let stop_notify = Event::new();
            let mut msg_window = MsgWindow::new(stop_notify.clone());

            // Start the message window task
            let task_handle = std::thread::spawn(move || {
                msg_window.task();
            });

            // Wait a bit to ensure the window is created and running
            std::thread::sleep(Duration::from_millis(100));

            // Signal the stop notification
            stop_notify.signal();

            // Wait for the task to finish
            task_handle
                .join()
                .expect("Message window task should finish");
        })
        .expect("Message window task should run within timeout");
    }
}
