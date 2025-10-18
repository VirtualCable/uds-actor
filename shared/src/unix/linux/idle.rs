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
// Code adapted from udsactor v4.x python code
use libloading::Library;
use std::cell::RefCell;
use std::os::raw::{c_int, c_ulong, c_void};
use std::{ptr, thread_local};

use anyhow::{Ok, Result};

use crate::log;

macro_rules! load_fn {
    ($lib:expr, $name:expr, $ty:ty) => {
        *$lib.get::<$ty>($name).ok()?
    };
}

#[repr(C)]
pub struct XScreenSaverInfo {
    window: c_ulong,
    state: c_int,
    kind: c_int,
    til_or_since: c_ulong,
    idle: c_ulong,
    event_mask: c_ulong,
}

struct IdleState {
    _xlib: Library, // Keep the library loaded, avoid drop
    _xss: Library,  // Keep the library loaded, avoid drop
    display: *mut c_void,
    info: *mut XScreenSaverInfo,
    x_connection_number: unsafe extern "C" fn(*mut c_void) -> c_int,
    x_default_root_window: unsafe extern "C" fn(*mut c_void) -> c_ulong,
    xss_screensaver_query_info: unsafe extern "C" fn(*mut c_void, c_ulong, *mut XScreenSaverInfo),
    x_free: unsafe extern "C" fn(*mut c_void) -> c_int,
}

thread_local! {
    static IDLE_STATE: RefCell<Option<IdleState>> = RefCell::new(None);
}

// Silent X IO error handler to avoid crashes (because on session close
// X server will be gone, but we will asking for idle time)
extern "C" fn silent_io_error_handler(_: *mut c_void) -> c_int {
    log::info!("X IO error — server may be dead");
    0 // no exit
}

fn is_display_alive(state: &IdleState) -> bool {
    let fd = unsafe { (state.x_connection_number)(state.display) };
    let mut pfd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    let res = unsafe { libc::poll(&mut pfd, 1, 0) };
    res >= 0 && (pfd.revents & libc::POLLHUP == 0)
}

pub(super) fn init_idle(seconds: u64) -> Result<()> {
    let success = IDLE_STATE.with(|cell| {
        // Already initialized
        if cell.borrow().is_some() {
            return Some(());
        }
        unsafe {
            let xlib = Library::new("libX11.so.6")
                .or_else(|_| Library::new("libX11.so"))
                .ok()?;
            let xss = Library::new("libXss.so.1")
                .or_else(|_| Library::new("libXss.so"))
                .ok()?;

            // use macro to load functions from libraries, copying types to avoid lifetime issues
            let x_open_display = load_fn!(
                xlib,
                b"XOpenDisplay",
                unsafe extern "C" fn(*const i8) -> *mut c_void
            );
            let x_default_root_window = load_fn!(
                xlib,
                b"XDefaultRootWindow",
                unsafe extern "C" fn(*mut c_void) -> c_ulong
            );
            let x_free= load_fn!(xlib, b"XFree", unsafe extern "C" fn(*mut c_void) -> c_int);
            let xss_alloc_info = load_fn!(
                xss,
                b"XScreenSaverAllocInfo",
                unsafe extern "C" fn() -> *mut XScreenSaverInfo
            );
            let xss_screensaver_query_info = load_fn!(
                xss,
                b"XScreenSaverQueryInfo",
                unsafe extern "C" fn(*mut c_void, c_ulong, *mut XScreenSaverInfo)
            );

            let display = x_open_display(ptr::null());
            if display.is_null() {
                return None;
            }
            let info = xss_alloc_info();
            if info.is_null() {
                return None;
            }

            // Set silent IO error handler
            let x_set_io_error_handler = load_fn!(
                xlib,
                b"XSetIOErrorHandler",
                unsafe extern "C" fn(unsafe extern "C" fn(*mut c_void) -> c_int)
            );
            x_set_io_error_handler(silent_io_error_handler);

            // XConnectionNumber for testing connection to X server before using it
            let x_connection_number = load_fn!(
                xlib,
                b"XConnectionNumber",
                unsafe extern "C" fn(*mut c_void) -> c_int
            );

            // XScreenSaverQueryExtension
            let xss_query_extension = load_fn!(
                xss,
                b"XScreenSaverQueryExtension",
                unsafe extern "C" fn(*mut c_void, *mut c_int, *mut c_int) -> c_int
            );

            // I no extension, return error
            let mut event_base: c_int = 0;
            let mut error_base: c_int = 0;
            if xss_query_extension(display, &mut event_base, &mut error_base) == 0 {
                x_free(info as *mut c_void);
                return None;
            }

            let state = IdleState {
                _xlib: xlib,
                _xss: xss,
                display,
                info,
                x_connection_number,
                x_default_root_window,
                xss_screensaver_query_info,
                x_free,
            };
            cell.replace(Some(state));
            Some(())
        }
    });
    if success.is_some() {
        // Set the screensaver timeout to desired seconds
        std::process::Command::new("xset")
            .arg("s")
            .arg(&seconds.to_string())
            .status()
            .ok();
        // Reset the screensaver
        std::process::Command::new("xset")
            .arg("s")
            .arg("reset")
            .status()
            .ok();
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to initialize idle state"))
    }
}

pub(super) fn get_idle() -> Result<std::time::Duration> {
    IDLE_STATE.with(|cell| {
        let borrow = cell.borrow();
        let Some(state) = borrow.as_ref() else {
            return Ok(std::time::Duration::from_secs(0));
        };

        if !is_display_alive(state) {
            log::debug!("Display connection is dead — skipping idle query");
            return Err(anyhow::anyhow!("Display connection is dead"));
        }

        unsafe {
            let root = (state.x_default_root_window)(state.display);
            (state.xss_screensaver_query_info)(state.display, root, state.info);
            if (*state.info).state == 1 {  // 1 = ScreenSaverActive
                return Ok(std::time::Duration::from_secs(3600 * 24 * 365 * 1000));  // A very large idle time
            }
            Ok(std::time::Duration::from_millis((*state.info).idle as u64))
        }
    })
}

#[allow(dead_code)]
pub fn shutdown_idle() {
    IDLE_STATE.with(|cell| {
        if let Some(state) = cell.borrow_mut().as_mut() {
            unsafe {
                if !state.info.is_null() {
                    (state.x_free)(state.info as *mut c_void);
                    state.info = ptr::null_mut();
                }
            }
        }
        cell.replace(None);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::log;

    #[test]
    fn test_get_idle() {
        crate::log::setup_logging("debug", crate::log::LogType::Tests);
        let _res = init_idle(32);
        // assert!(res.is_ok());
        for _i in 0..32 {
            let idle = get_idle().unwrap();
            log::info!("Idle time: {} seconds", idle.as_secs());
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        shutdown_idle();
    }
}
