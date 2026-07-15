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
use std::{
    backtrace::Backtrace,
    fs::{self, OpenOptions},
    io::{self, Write},
    panic,
    path::PathBuf,
    sync::OnceLock,
};
use tracing_subscriber::{
    EnvFilter, Layer, Registry, fmt, layer::SubscriberExt, reload, util::SubscriberInitExt,
};

use crate::log_forward::LogForwardLayer;

#[cfg(target_os = "windows")]
use crate::windows::eventlog::EventLogLayer;

// Reexport to avoid using crate names for tracing
pub use tracing::{debug, error, info, trace, warn};

static LOGGER_INIT: OnceLock<()> = OnceLock::new();
static RELOAD_HANDLE: OnceLock<reload::Handle<EnvFilter, Registry>> = OnceLock::new();
static ACTIVE_LOG_LEVEL: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(3); // Default Info (3)

pub fn get_active_log_level() -> tracing::Level {
    match ACTIVE_LOG_LEVEL.load(std::sync::atomic::Ordering::Relaxed) {
        1 => tracing::Level::ERROR,
        2 => tracing::Level::WARN,
        3 => tracing::Level::INFO,
        4 => tracing::Level::DEBUG,
        5 => tracing::Level::TRACE,
        _ => tracing::Level::INFO,
    }
}

pub fn parse_level(s: &str) -> Option<tracing::Level> {
    match s.to_lowercase().as_str() {
        "trace" => Some(tracing::Level::TRACE),
        "debug" => Some(tracing::Level::DEBUG),
        "info" => Some(tracing::Level::INFO),
        "warn" | "warning" => Some(tracing::Level::WARN),
        "error" => Some(tracing::Level::ERROR),
        _ => None,
    }
}

fn set_active_level_from_str(level: &str) {
    let level_num = match level.to_lowercase().as_str() {
        "error" => 1,
        "warn" | "warning" => 2,
        "info" => 3,
        "debug" => 4,
        "trace" => 5,
        _ => 3,
    };
    ACTIVE_LOG_LEVEL.store(level_num, std::sync::atomic::Ordering::Relaxed);
}

struct RotatingWriter {
    path: PathBuf,
    max_size: u64,    // Max size in bytes before rotation
    max_files: usize, // Number of rotations to keep
}

impl RotatingWriter {
    fn rotate_if_needed(&self) -> io::Result<()> {
        if let Ok(meta) = fs::metadata(&self.path)
            && meta.len() >= self.max_size
        {
            // Remove last if needed
            if self.max_files > 1 {
                let last = self.path.with_extension(format!("log.{}", self.max_files));
                let _ = fs::remove_file(&last);
                // Rename in reverse order
                for i in (1..self.max_files).rev() {
                    let src = self.path.with_extension(format!("log.{}", i));
                    let dst = self.path.with_extension(format!("log.{}", i + 1));
                    let _ = fs::rename(&src, &dst);
                }
                // Rename current to .log.1
                let rotated = self.path.with_extension("log.1");
                let _ = fs::rename(&self.path, rotated);
            } else {
                // if max_files is 1, just remove current
                let _ = fs::remove_file(&self.path);
            }
        }
        Ok(())
    }
}

impl<'a> fmt::MakeWriter<'a> for RotatingWriter {
    type Writer = fs::File;

    fn make_writer(&'a self) -> Self::Writer {
        // Rotate if needed
        let _ = self.rotate_if_needed();
        // Ensure the parent directory exists. This is mostly a no-op for the
        // well-known default locations (C:\Windows\Temp, /var/log/udsactor)
        // but protects against custom paths that may not exist yet.
        if let Some(parent) = self.path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        // Always open in append mode, creating it if it doesn't exist
        // If self.path cannot be opened, try with one in temp dir
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .unwrap_or_else(|e| {
                let temp_path = std::env::temp_dir().join("udsactor-fallback.log");
                if let Some(parent) = temp_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                eprintln!(
                    "udsactor: could not open '{}': {} (falling back to '{}')",
                    self.path.display(),
                    e,
                    temp_path.display()
                );
                OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&temp_path)
                    .unwrap_or_else(|e| panic!("Failed to open log file {:?}: {}", temp_path, e))
            })
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum LogType {
    Client,
    Service,
    Config,
    Tests,
}

impl std::fmt::Display for LogType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogType::Client => write!(f, "client"),
            LogType::Service => write!(f, "service"),
            LogType::Config => write!(f, "config"),
            LogType::Tests => write!(f, "tests"),
        }
    }
}

// Our log system wil also hook panics to log them
pub fn setup_panic_hook() {
    panic::set_hook(Box::new(|info| {
        let temp_log = std::env::temp_dir().join("udsactor-panic.log");
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&temp_log)
            .unwrap();

        // Extraer payload del panic
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Non-string panic payload".to_string()
        };

        // Localización
        let loc = if let Some(location) = info.location() {
            format!("{}:{}", location.file(), location.line())
        } else {
            "unknown location".to_string()
        };

        // Backtrace
        let bt = Backtrace::capture();

        writeln!(f, "Panic occurred at {}: {}", loc, msg).ok();
        writeln!(f, "Backtrace:\n{:?}", bt).ok();

        error!("Guru Meditation (😕): {} at {}", msg, loc);
        error!("Backtrace:\n{:?}", bt);
    }));
}

/// Resolve the **directory** (no file name) where logs should be written when
/// `UDSACTOR_<TYPE>_LOG_PATH` is not set.
///
/// Defaults per platform:
///
/// - **Windows service / config**: `std::env::temp_dir()` -> `C:\Windows\Temp`
///   (the process runs as LocalSystem, so its TEMP maps to that location).
/// - **Windows client**: `std::env::temp_dir()` -> `%USERPROFILE%\AppData\Local\Temp`
///   (the `udsactor-client.exe` is launched per-user by the user themselves,
///   so its TEMP resolves correctly to that user's temp folder).
/// - **Linux / macOS service**: tries `/var/log/udsactor` (created if possible),
///   falling back to `/tmp` if not writable.
/// - **Linux / macOS client**: `$TMPDIR` -> `/tmp`.
///
/// See `notes/env-vars-logging.md` for the full list of override env vars.
#[cfg_attr(target_os = "windows", allow(unused_variables))]
fn default_log_dir(log_type: &LogType) -> String {
    #[cfg(target_os = "windows")]
    {
        // temp_dir() already returns the *current process's* TEMP:
        //   - LocalSystem (service) => C:\Windows\Temp
        //   - User (client)        => %USERPROFILE%\AppData\Local\Temp
        let _ = log_type; // not used on Windows; suppress unused-variable warning
        std::env::temp_dir().to_string_lossy().into_owned()
    }

    #[cfg(target_family = "unix")]
    {
        let service_mode = matches!(log_type, LogType::Service | LogType::Config);
        if service_mode {
            // Try /var/log/udsactor first; if we cannot write there, fall back to /tmp.
            let var_log = std::path::PathBuf::from("/var/log/udsactor");
            if std::fs::create_dir_all(&var_log).is_ok() {
                // Probe writability with a tiny file (cheap and reliable).
                let probe = var_log.join(".udsactor-write-probe");
                if std::fs::write(&probe, b"ok")
                    .and_then(|_| std::fs::remove_file(&probe))
                    .is_ok()
                {
                    return var_log.to_string_lossy().into_owned();
                }
            }
            return "/tmp".to_string();
        }
        // Client: trust the user's TMPDIR / fall back to /tmp.
        std::env::temp_dir().to_string_lossy().into_owned()
    }
}

pub fn setup_logging(level: &str, log_type: LogType) {
    let (level_key, log_path, use_datetime, log_name) = (
        format!("UDSACTOR_{}_LOG_LEVEL", log_type.to_string().to_uppercase()),
        format!("UDSACTOR_{}_LOG_PATH", log_type.to_string().to_uppercase()),
        format!(
            "UDSACTOR_{}_LOG_USE_DATETIME",
            log_type.to_string().to_uppercase()
        ),
        format!("udsactor-{}", log_type.to_string().to_lowercase()),
    );

    let level = std::env::var(level_key).unwrap_or_else(|_| level.to_string());
    let log_path = std::env::var(log_path).unwrap_or_else(|_| default_log_dir(&log_type));
    let use_datetime: bool = std::env::var(use_datetime)
        .unwrap_or_else(|_| "false".into())
        .to_lowercase()
        .parse()
        .unwrap_or(false);

    let log_name = if use_datetime {
        let op = crate::system::new_system();
        let computer_name = op.get_computer_name().unwrap_or_else(|_| "unknown".into());
        let dt = chrono::Local::now();
        format!(
            "{}-{}-{}",
            log_name,
            computer_name,
            dt.format("%Y%m%d-%H%M%S")
        )
    } else {
        log_name.to_string()
    } + ".log";

    // Best-effort: make sure the log directory exists. We don't fail setup if
    // the directory cannot be created (RotatingWriter falls back to
    // `temp_dir()/udsactor-fallback.log`), but we try to surface the situation
    // to the operator via an early warning so they know where to look.
    let full_log_path = std::path::Path::new(&log_path).join(&log_name);
    if let Some(parent) = full_log_path.parent() {
        match std::fs::create_dir_all(parent) {
            Ok(()) => {
                // Announce to stderr so the operator always knows the path,
                // even when stdout redirection strips log output.
                eprintln!("udsactor log file: {}", full_log_path.display());
            }
            Err(e) => {
                eprintln!(
                    "udsactor: could not create log directory '{}': {} (will fall back to a temp dir file)",
                    parent.display(),
                    e
                );
            }
        }
    }

    LOGGER_INIT.get_or_init(|| {
        set_active_level_from_str(&level);
        let env_filter = EnvFilter::new(level.clone());
        let (reload_layer, handle) = reload::Layer::<EnvFilter, Registry>::new(env_filter);

        let _ = RELOAD_HANDLE.set(handle);

        let main_layer = fmt::layer()
            .with_writer(RotatingWriter {
                path: std::path::Path::new(&log_path).join(log_name),
                max_size: 16 * 1024 * 1024, // 16 MB
                max_files: 2,
            })
            .with_ansi(false)
            .with_target(true)
            .with_level(true)
            .with_thread_ids(level == "debug" || level == "trace")
            .with_filter(reload_layer);

        // Stderr layer: emits colored, line-numbered output to stderr.
        // Useful under systemd / launchd, which capture stderr into the
        // journal / log show. Enabled for service and client (both run as
        // long-lived processes). Config is intentionally excluded because
        // it's a one-shot GUI tool.
        //
        // On debug builds we force-enable it and bump verbosity. On release
        // it's also enabled by default; disable with
        // UDSACTOR_<TYPE>_STDERR_DISABLE=true.
        let stderr_disable_key = format!(
            "UDSACTOR_{}_STDERR_DISABLE",
            log_type.to_string().to_uppercase()
        );
        let stderr_enabled = matches!(log_type, LogType::Service | LogType::Client)
            && std::env::var(&stderr_disable_key)
                .ok()
                .map(|v| !matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
                .unwrap_or(true);

let stderr_level = if !stderr_enabled {
    // Effectively disable: filter out everything.
    "off"
} else if cfg!(debug_assertions) {
    "debug"
} else {
    // On release, match the file log level for consistency.
    level.as_str()
};

// Stderr layer: always added, controlled by EnvFilter so the type stays
// uniform across the conditional branch above.
let main_layer = main_layer.and_then(
    fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(true)
        .with_target(true)
        .with_level(true)
        .with_thread_ids(true)
        .with_file(cfg!(debug_assertions))
        .with_line_number(cfg!(debug_assertions))
        .with_filter(EnvFilter::new(stderr_level)),
);

// Optional forwarder: only `LogType::Service` actually forwards
// (see `LogForwardLayer::for_type`); for everything else this returns
// a layer whose `enabled()` is false, which is a no-op.
let main_layer = main_layer.and_then(LogForwardLayer::for_type(&log_type));

#[cfg(target_os = "windows")]
let main_layer = main_layer.and_then(EventLogLayer::for_type(&log_type));

tracing_subscriber::registry()
    .with(main_layer)
    .try_init()
    .ok();

        // Setup panic hook, not if testing
        if log_type != LogType::Tests {
            setup_panic_hook();
        }
    });
}

pub fn set_log_level(level: &str) {
    // If an environment variable is setting the log level explicitly, it has precedence.
    if std::env::var("UDSACTOR_SERVICE_LOG_LEVEL").is_ok()
        || std::env::var("UDSACTOR_CLIENT_LOG_LEVEL").is_ok()
        || std::env::var("UDSACTOR_CONFIG_LOG_LEVEL").is_ok()
    {
        return;
    }

    set_active_level_from_str(level);
    // Note: Changing log level at runtime is not directly supported by tracing_subscriber.
    // This is a workaround by re-initializing the subscriber with the new level.
    if let Some(handle) = RELOAD_HANDLE.get() {
        let new_filter = EnvFilter::new(level);
        if let Err(e) = handle.modify(|f| *f = new_filter) {
            eprintln!("Failed to reload log level: {}", e);
        }
    } else {
        eprintln!("Logger not initialized yet");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    #[ignore] // Ignored because it requires Windows service environment
    fn test_logging_on_network_path() {
        unsafe { std::env::set_var("UDSACTOR_TESTS_LOG_PATH", r"\\172.27.1.45\shared") }
        setup_logging("debug", LogType::Tests);
        info!("This is a test log entry on network path");
        debug!("Debug entry");
        warn!("Warning entry");
        error!("Error entry");
        trace!("Trace entry");
    }

    #[test]
    fn test_logging_on_default_path() {
        setup_logging("debug", LogType::Tests);
        info!("This is a test log entry on default path");
        debug!("Debug entry");
        warn!("Warning entry");
        error!("Error entry");
        trace!("Trace entry");
    }

    #[test]
    fn test_logging_with_datetime() {
        unsafe {
            std::env::set_var("UDSACTOR_TESTS_LOG_PATH", std::env::temp_dir());
            std::env::set_var("UDSACTOR_TESTS_LOG_USE_DATETIME", "true");
        }
        setup_logging("debug", LogType::Tests);
        info!("This is a test log entry with datetime in filename");
        debug!("Debug entry");
        warn!("Warning entry");
        error!("Error entry");
        trace!("Trace entry");
    }

    #[test]
    #[ignore] // Ignored because it generates a lot of log data on console
    fn test_logging_rotation() {
        let temp_dir = std::env::temp_dir();
        unsafe { std::env::set_var("UDSACTOR_TESTS_LOG_PATH", &temp_dir) }
        setup_logging("debug", LogType::Tests);
        let log_file = temp_dir.join("udsactor-tests.log");
        // Write enough logs to exceed 16MB
        for i in 0..20000 {
            info!("Log entry number: {} - {}", i, "A".repeat(1024)); // Each entry ~1KB
        }
        // Check if log file exists
        assert!(log_file.exists());
        // Check if rotated file exists
        let rotated_file = temp_dir.join("udsactor-tests.log.1");
        assert!(rotated_file.exists()); // Rotated file should exist
        // Check if log file has been rotated
        let meta = fs::metadata(&log_file).unwrap();
        assert!(meta.len() < 16 * 1024 * 1024); // Current log file should be less than 16MB
    }

    #[test]
    fn test_tracing_level_order() {
        assert!(tracing::Level::ERROR < tracing::Level::WARN);
    }
}
