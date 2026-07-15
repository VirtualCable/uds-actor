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

//! `tracing-subscriber::Layer` that forwards events to the Windows Event
//! Viewer using the `advapi32` EventLog API.
//!
//! Layer contract:
//!
//! - Only active for `LogType::Service` (same policy as the broker
//!   forwarder). Client and Config are no-ops.
//! - Respects `UDSACTOR_<TYPE>_EVENTLOG_LEVEL` (default `info`) and
//!   `UDSACTOR_<TYPE>_EVENTLOG_DISABLE` (default `false`).
//! - Maps `tracing::Level` to `EVENTLOG_*_TYPE`: ERROR -> error (red),
//!   WARN -> warning (yellow), everything else -> information.
//! - The EventLog handle is registered lazily on the first event and
//!   cached in a `OnceLock`. If `RegisterEventSourceW` fails at startup
//!   the layer degrades to a no-op and prints a one-line warning to
//!   stderr; we never panic.
//! - The caller is expected to invoke [`shutdown`] once at process exit
//!   to release the handle. Skipping `shutdown()` is safe -- the OS
//!   reclaims the handle when the process exits anyway.

use std::sync::OnceLock;

use tracing::{Event, Subscriber};
use tracing_subscriber::{Layer, layer::Context, registry::LookupSpan};
use windows::{
    Win32::{
        Foundation::HANDLE,
        System::EventLog::{
            DeregisterEventSource, EVENTLOG_ERROR_TYPE, EVENTLOG_INFORMATION_TYPE,
            EVENTLOG_WARNING_TYPE, RegisterEventSourceW, ReportEventW,
        },
    },
    core::PCWSTR,
};

use crate::log::LogType;

/// Source name registered with the EventLog. Windows accepts unregistered
/// sources (entries are still written, just with a generic category).
const SOURCE_NAME: &str = "UDS Actor Service";

/// Newtype wrapper that marks an EventLog `HANDLE` as `Send + Sync`. The
/// underlying HANDLE is just a numeric kernel handle, so this is safe to
/// share across threads (the Win32 EventLog APIs are themselves thread
/// safe).
#[derive(Clone, Copy)]
struct EventLogHandle(HANDLE);

// HANDLE is `*mut c_void` and is `!Send + !Sync` by default; EventLog
// APIs are thread-safe, so this assertion is sound.
unsafe impl Send for EventLogHandle {}
unsafe impl Sync for EventLogHandle {}

static EVENTLOG_HANDLE: OnceLock<EventLogHandle> = OnceLock::new();
static REGISTER_FAILED: OnceLock<()> = OnceLock::new();

/// `tracing-subscriber::Layer` that forwards events whose level is `>=`
/// the configured threshold to the Windows EventLog.
pub struct EventLogLayer {
    enabled: bool,
}

impl EventLogLayer {
    /// Build a layer for the given component type. Returns a layer whose
    /// `enabled()` is `false` for anything that is not `LogType::Service`.
    pub fn for_type(log_type: &LogType) -> Self {
        let disable_key = format!(
            "UDSACTOR_{}_EVENTLOG_DISABLE",
            log_type.to_string().to_uppercase()
        );

        // Hard-gate by log type first: only the service writes to the
        // EventLog. The client and config tools don't need it.
        let type_allowed = matches!(log_type, LogType::Service);

        let enabled = type_allowed
            && std::env::var(&disable_key)
                .ok()
                .map(|v| !matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
                .unwrap_or(true);

        Self { enabled }
    }
}



fn report_type_for(level: tracing::Level) -> u16 {
    match level {
        tracing::Level::ERROR => EVENTLOG_ERROR_TYPE.0,
        tracing::Level::WARN => EVENTLOG_WARNING_TYPE.0,
        _ => EVENTLOG_INFORMATION_TYPE.0,
    }
}

fn register_event_source_in_registry() {
    use windows::Win32::System::Registry::*;
    use widestring::U16CString;

    unsafe {
        let key_path = U16CString::from_str_truncate(
            r"SYSTEM\CurrentControlSet\Services\EventLog\Application\UDS Actor Service"
        );
        let mut hkey = HKEY::default();
        let status = RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            windows::core::PCWSTR(key_path.as_ptr()),
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut hkey,
            None,
        );
        if status.is_ok() {
            // Write EventMessageFile (REG_EXPAND_SZ) pointing to EventCreate.exe
            let msg_file = U16CString::from_str_truncate(r"%SystemRoot%\System32\EventCreate.exe");
            let msg_file_bytes = std::slice::from_raw_parts(
                msg_file.as_ptr() as *const u8,
                (msg_file.len() + 1) * 2,
            );
            let _ = RegSetValueExW(
                hkey,
                windows::core::PCWSTR(U16CString::from_str_truncate("EventMessageFile").as_ptr()),
                None,
                REG_EXPAND_SZ,
                Some(msg_file_bytes),
            );

            // Write TypesSupported (REG_DWORD)
            let types_supported: u32 = 7;
            let types_bytes = std::slice::from_raw_parts(
                &types_supported as *const u32 as *const u8,
                4,
            );
            let _ = RegSetValueExW(
                hkey,
                windows::core::PCWSTR(U16CString::from_str_truncate("TypesSupported").as_ptr()),
                None,
                REG_DWORD,
                Some(types_bytes),
            );

            let _ = RegCloseKey(hkey);
        }
    }
}

/// Returns the EventLog handle, registering it on first call.
/// Returns `None` if registration previously failed (in which case the
/// layer silently skips subsequent events).
fn get_or_register_handle() -> Option<EventLogHandle> {
    if REGISTER_FAILED.get().is_some() {
        return None;
    }
    if let Some(h) = EVENTLOG_HANDLE.get() {
        return Some(*h);
    }
    // Register custom source in the registry so Event Viewer formats it correctly
    register_event_source_in_registry();

    let wide = encode_wide_with_nul(SOURCE_NAME)?;
    // SAFETY: `wide` is a valid wide C string with trailing nul, lives
    // for the duration of this call. Passing `None` for the server name
    // means "local computer".
    let res = unsafe { RegisterEventSourceW(PCWSTR::null(), PCWSTR(wide.as_ptr())) };
    match res {
        Ok(h) => {
            let wrapped = EventLogHandle(h);
            // First writer wins; both races resolve to the same source.
            let _ = EVENTLOG_HANDLE.set(wrapped);
            EVENTLOG_HANDLE.get().copied()
        }
        Err(e) => {
            // Don't spam stderr on every event; one warning is enough.
            if REGISTER_FAILED.get().is_none() {
                eprintln!(
                    "udsactor: RegisterEventSourceW(\"{}\") failed: {e:?} \
                     (EventLog forwarding disabled for this run)",
                    SOURCE_NAME
                );
            }
            let _ = REGISTER_FAILED.set(());
            None
        }
    }
}

/// Encode a `&str` as a UTF-16 buffer with a trailing nul byte.
/// `SOURCE_NAME` is a constant without interior nuls, so this is
/// infallible in practice; we keep the `Option` for defensive coding.
fn encode_wide_with_nul(s: &str) -> Option<Vec<u16>> {
    if s.contains('\0') {
        return None;
    }
    let mut wide: Vec<u16> = s.encode_utf16().collect();
    wide.push(0);
    Some(wide)
}

impl<S> Layer<S> for EventLogLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn enabled(&self, metadata: &tracing::Metadata<'_>, _ctx: Context<'_, S>) -> bool {
        self.enabled && *metadata.level() <= crate::log::get_active_log_level()
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let Some(handle) = get_or_register_handle() else {
            return;
        };

        // Collect the formatted message (same approach as log_forward.rs).
        let level = *event.metadata().level();
        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let message = visitor.0;
        if message.is_empty() {
            return;
        }

        // Build the wide string for ReportEventW. The API takes a slice
        // of PCWSTR (one per insertion string). We pass exactly one: the
        // full message.
        let wide = message.encode_utf16().chain([0u16]).collect::<Vec<u16>>();
        let string_ptr = PCWSTR(wide.as_ptr());
        let strings = [string_ptr];
        let report_type = report_type_for(level);

        // SAFETY: `strings` points to a single PCWSTR backed by `wide`,
        // which is alive for the duration of this call. The slice length
        // (1) is what ReportEventW uses to compute wNumStrings.
        let res = unsafe {
            ReportEventW(
                handle.0,
                windows::Win32::System::EventLog::REPORT_EVENT_TYPE(report_type),
                0,    // wCategory
                1,    // dwEventID
                None, // lpUserSid
                0,    // dwDataSize
                Some(&strings),
                None, // lpRawData
            )
        };
        if let Err(e) = res {
            // Don't recurse into tracing -- write to stderr as a last resort.
            eprintln!("udsactor: ReportEventW failed: {e:?}");
        }
    }
}

/// Release the EventLog handle, if any. Safe to call multiple times.
/// Should be invoked once at service shutdown; the OS also reclaims
/// the handle on process exit so this is a best-effort cleanup.
pub fn shutdown() {
    if let Some(handle) = EVENTLOG_HANDLE.get() {
        // SAFETY: the handle was returned by RegisterEventSourceW and
        // is owned by us. DeregisterEventSource is safe to call once.
        unsafe {
            let _ = DeregisterEventSource(handle.0);
        }
    }
}

#[derive(Default)]
struct MessageVisitor(String);

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let s = format!("{value:?}");
            self.0.push_str(s.trim_matches('"'));
        } else {
            use std::fmt::Write;
            let _ = write!(self.0, " {}={:?}", field.name(), value);
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.0.push_str(value);
        } else {
            use std::fmt::Write;
            let _ = write!(self.0, " {}={}", field.name(), value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::parse_level;

    #[test]
    fn parse_levels() {
        assert_eq!(parse_level("info"), Some(tracing::Level::INFO));
        assert_eq!(parse_level("DEBUG"), Some(tracing::Level::DEBUG));
        assert_eq!(parse_level("warn"), Some(tracing::Level::WARN));
        assert_eq!(parse_level("nope"), None);
    }

    #[test]
    fn report_type_mapping() {
        assert_eq!(report_type_for(tracing::Level::ERROR), 1);
        assert_eq!(report_type_for(tracing::Level::WARN), 2);
        assert_eq!(report_type_for(tracing::Level::INFO), 4);
        assert_eq!(report_type_for(tracing::Level::DEBUG), 4);
        assert_eq!(report_type_for(tracing::Level::TRACE), 4);
    }

    #[test]
    fn for_type_only_enables_service() {
        for log_type in [LogType::Client, LogType::Config, LogType::Tests] {
            let layer = EventLogLayer::for_type(&log_type);
            assert!(
                !layer.enabled,
                "EventLog layer must be disabled for {log_type:?}"
            );
        }
        let layer = EventLogLayer::for_type(&LogType::Service);
        assert!(layer.enabled, "EventLog layer must be enabled for Service");
    }

    #[test]
    fn encode_wide_includes_nul_terminator() {
        let wide = encode_wide_with_nul("hi").expect("encode should succeed");
        assert_eq!(wide, vec![b'h' as u16, b'i' as u16, 0u16]);
    }

    #[test]
    fn encode_wide_rejects_interior_nul() {
        assert!(encode_wide_with_nul("a\0b").is_none());
    }

    #[test]
    fn shutdown_is_idempotent() {
        // Calling shutdown() with no prior registration must not panic.
        shutdown();
        shutdown();
    }
}