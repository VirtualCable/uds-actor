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
use libloading::{Library, Symbol};
use std::cell::RefCell;
use std::os::raw::{c_int, c_ulong, c_void};
use std::{ptr, thread_local};

use anyhow::Result;

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
    _xlib: Library,
    _xss: Library,
    display: *mut c_void,
    info: *mut XScreenSaverInfo,
    x_default_root_window: unsafe extern "C" fn(*mut c_void) -> c_ulong,
    x_screensaver_query_info: unsafe extern "C" fn(*mut c_void, c_ulong, *mut XScreenSaverInfo),
    x_free: unsafe extern "C" fn(*mut c_void) -> c_int,
}

thread_local! {
    static IDLE_STATE: RefCell<Option<IdleState>> = RefCell::new(None);
}

pub(super) fn init_idle() -> Result<()> {
    let success = IDLE_STATE.with(|cell| {
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

            let x_open_display: Symbol<unsafe extern "C" fn(*const i8) -> *mut c_void> =
                xlib.get(b"XOpenDisplay").ok()?;
            let x_default_root_window: Symbol<unsafe extern "C" fn(*mut c_void) -> c_ulong> =
                xlib.get(b"XDefaultRootWindow").ok()?;
            let x_free: Symbol<unsafe extern "C" fn(*mut c_void) -> c_int> =
                xlib.get(b"XFree").ok()?;

            let xss_alloc_info: Symbol<unsafe extern "C" fn() -> *mut XScreenSaverInfo> =
                xss.get(b"XScreenSaverAllocInfo").ok()?;
            let xss_query_info: Symbol<
                unsafe extern "C" fn(*mut c_void, c_ulong, *mut XScreenSaverInfo),
            > = xss.get(b"XScreenSaverQueryInfo").ok()?;

            let display = x_open_display(ptr::null());
            if display.is_null() {
                return None;
            }
            let info = xss_alloc_info();
            if info.is_null() {
                return None;
            }

            // 👇 copiamos los punteros de función a variables locales
            let x_default_root_window = *x_default_root_window;
            let x_screensaver_query_info = *xss_query_info;
            let x_free = *x_free;

            let state = IdleState {
                _xlib: xlib,
                _xss: xss,
                display,
                info,
                x_default_root_window,
                x_screensaver_query_info,
                x_free,
            };
            cell.replace(Some(state));
            Some(())
        }
    });
    if success.is_some() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("Failed to initialize idle state"))
    }
}

pub(super) fn get_idle() -> f64 {
    IDLE_STATE.with(|cell| {
        let borrow = cell.borrow();
        let Some(state) = borrow.as_ref() else {
            return 0.0;
        };
        unsafe {
            let root = (state.x_default_root_window)(state.display);
            (state.x_screensaver_query_info)(state.display, root, state.info);
            if (*state.info).state == 1 {
                return 3600.0 * 100.0 * 1000.0;
            }
            (*state.info).idle as f64 / 1000.0
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

    #[test]
    fn test_get_idle() {
        crate::log::setup_logging("debug", crate::log::LogType::Tests);
        let res =init_idle();
        assert!(res.is_ok());
        let idle = get_idle();
        println!("Idle time: {} seconds", idle);
        assert!(idle >= 0.0);
        shutdown_idle();
    }
}
