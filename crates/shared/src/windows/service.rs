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
use std::sync::{Arc, OnceLock};

use windows::{
    Win32::Foundation::*, Win32::System::Services::*, Win32::System::Threading::*, core::*,
};

use anyhow::Result;

use crate::{log::{debug, info},service::AsyncServiceTrait, sync::OnceSignal};

const SERVICE_NAME: PCWSTR = w!("RustExampleService");

static LAUNCHER: OnceLock<Arc<dyn AsyncServiceTrait>> = OnceLock::new();

#[derive(Clone, Debug)]
struct ServiceContext {
    status_handle: SERVICE_STATUS_HANDLE,
    stop_event: HANDLE,
    async_stop: OnceSignal,
    exit_code: u32,
}

unsafe impl Send for ServiceContext {}
unsafe impl Sync for ServiceContext {}

impl ServiceContext {
    fn new(async_stop: OnceSignal) -> Self {
        Self {
            status_handle: SERVICE_STATUS_HANDLE(std::ptr::null_mut()),
            stop_event: unsafe { CreateEventW(None, true, false, None).unwrap() },
            async_stop,
            exit_code: NO_ERROR.0,
        }
    }

    pub fn report_status(
        &self,
        current_state: SERVICE_STATUS_CURRENT_STATE,
        checkpoint: u32,
        wait_hint_ms: u32,
    ) -> Result<()> {
        debug!(
            "Reporting status: {:?}, checkpoint: {}, wait_hint: {}",
            current_state, checkpoint, wait_hint_ms
        );
        unsafe {
            let status = SERVICE_STATUS {
                dwServiceType: SERVICE_USER_OWN_PROCESS,
                dwCurrentState: current_state,
                dwControlsAccepted: match current_state {
                    SERVICE_RUNNING => SERVICE_ACCEPT_STOP,
                    SERVICE_STOP_PENDING => SERVICE_ACCEPT_STOP,
                    _ => 0,
                },
                dwWin32ExitCode: self.exit_code,
                dwServiceSpecificExitCode: 0,
                dwCheckPoint: checkpoint,
                dwWaitHint: wait_hint_ms,
            };

            SetServiceStatus(self.status_handle, &status)?;
        }
        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        info!("Service stop requested");
        unsafe {
            SetEvent(self.stop_event)?;
        }
        Ok(())
    }

    pub fn wait_for_stop(&self, timeout_ms: u32) -> Result<WAIT_EVENT> {
        debug!("Waiting for service stop...");
        unsafe {
            let res = WaitForSingleObject(self.stop_event, timeout_ms);
            Ok(res)
        }
    }

    pub fn close(&mut self) -> Result<()> {
        debug!("Closing service context...");
        if !self.stop_event.is_invalid() {
            unsafe {
                CloseHandle(self.stop_event)?;
            }
            self.stop_event = HANDLE::default();
        }
        Ok(())
    }
}

// Handler signature OK; returns DWORD
extern "system" fn service_handler(
    ctrl: u32,
    _event_type: u32,
    _event_data: *mut std::ffi::c_void,
    context: *mut std::ffi::c_void,
) -> u32 {
    let ctx = unsafe { &mut *(context as *mut ServiceContext) };

    match ctrl {
        SERVICE_CONTROL_STOP => {
            // Spawn a thread that does the work and notifies progress
            let mut checkpoint = 1;
            ctx.async_stop.set();
            while ctx.wait_for_stop(100).unwrap() == WAIT_TIMEOUT {
                std::thread::sleep(std::time::Duration::from_millis(100));
                // Notify the SCM that we're still in STOP_PENDING
                let _ = ctx.report_status(SERVICE_STOP_PENDING, checkpoint, 10000);
                checkpoint += 1;
            }
        }
        SERVICE_CONTROL_INTERROGATE => {
            let _ = ctx.report_status(SERVICE_RUNNING, 0, 0);
        }
        _ => {}
    }
    NO_ERROR.0
}

extern "system" fn service_main(_argc: u32, _argv: *mut PWSTR) {
    unsafe {
        info!("Service main started");
        let launcher = LAUNCHER.get().expect("Launcher not set");

        // Register the service control handler, with our context
        let mut ctx = ServiceContext::new(launcher.get_stop());

        let ctx_ptr: *mut ServiceContext = &mut ctx;
        ctx.status_handle = match RegisterServiceCtrlHandlerExW(
            SERVICE_NAME,
            Some(service_handler),
            Some(ctx_ptr as *mut _),
        ) {
            Ok(h) => h,
            Err(_) => return,
        };

        let _ = ctx.report_status(SERVICE_START_PENDING, 0, 3000);

        // Something here to initialize the service...

        let _ = ctx.report_status(SERVICE_RUNNING, 0, 0);

        // Launch a thread that does some work and then signals the stop event
        let ctx_thread = ctx.clone();
        std::thread::spawn(move || {
            debug!("Service worker thread started");
            // Execute async work
            launcher.run(ctx_thread.async_stop.clone());
            // When done, signal the service to stop
            ctx_thread.stop().unwrap();
        });

        // Wait until the stop event is signaled
        let _ = ctx.wait_for_stop(INFINITE);

        // If restart is requested, set exit_code to ERROR_SERVICE_SPECIFIC_ERROR
        if launcher.should_restart() {
            ctx.exit_code = ERROR_SERVICE_SPECIFIC_ERROR.0;
        } else {
            ctx.exit_code = NO_ERROR.0;
        }

        let _ = ctx.report_status(SERVICE_STOPPED, 0, 0);
        let _ = ctx.close();
    }
}

pub fn run_service<L: AsyncServiceTrait>(launcher: L) -> Result<()> {
    debug!("Running windows service...");
    LAUNCHER
        .set(Arc::new(launcher))
        .map_err(|_| anyhow::anyhow!("Launcher already set"))?;

    // Service Table: StartServiceCtrlDispatcherW(*const SERVICE_TABLE_ENTRYW) -> Result<()>
    let table = [
        SERVICE_TABLE_ENTRYW {
            lpServiceName: PWSTR(SERVICE_NAME.0 as *mut _),
            lpServiceProc: Some(service_main),
        },
        SERVICE_TABLE_ENTRYW {
            lpServiceName: PWSTR::null(),
            lpServiceProc: None,
        },
    ];

    unsafe {
        StartServiceCtrlDispatcherW(table.as_ptr())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stop_signals_notify_and_event() {
        let async_stop = OnceSignal::new();
        let mut ctx = ServiceContext::new(async_stop.clone());
        ctx.status_handle = SERVICE_STATUS_HANDLE::default(); // dummy

        let ctx_ptr: *mut ServiceContext = &mut ctx;

        // Set up a thread to wait for the notify
        let notify = ctx.async_stop.clone();

        let ctx_clone = ctx.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all() // Enable timers, I/O, etc.
                .build()
                .unwrap();

            rt.block_on(async {
                notify.wait().await;
            });
            ctx_clone.stop().unwrap();
        });

        // Wait a bit to ensure the thread is waiting
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Simulate the service control handler receiving a STOP command
        service_handler(
            SERVICE_CONTROL_STOP,
            0,
            std::ptr::null_mut(),
            ctx_ptr as *mut _,
        );

        // Event should be set, check with WaitForSingleObject
        let wait_result = ctx.wait_for_stop(1);
        assert!(wait_result.is_ok());
        assert_eq!(wait_result.unwrap(), WAIT_OBJECT_0);

        // We only reach here if stop is called
        if ctx.wait_for_stop(1000).unwrap() != WAIT_OBJECT_0 {
            panic!("Stop event was not signaled");
        }

        let _ = ctx.close();
    }
}
