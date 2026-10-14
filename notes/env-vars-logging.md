# UDS Actor — Logging environment variables

This document lists every environment variable that influences logging in the
Rust UDS Actor (service, client, config tool) and explains how each one is
resolved.

`<TYPE>` is one of:

| `<TYPE>` value | Component                    | Binary (`*.exe` / `udsactor-*`)       |
| --- | --- | --- |
| `SERVICE` | Service (`UDSActorService`) | `udsactor-service`              |
| `CLIENT`  | Per-user client              | `udsactor-client`               |
| `CONFIG`  | Config tool                  | `udsactor-config`               |
| `TESTS`   | Internal test runner         | (used by `cargo test`)          |

> The variables are inspected in this order, and the **first one that is set
> wins**. Everything case-sensitively maps to the lowercase `LogType`
> (`SERVICE` → `service`, `CLIENT` → `client`, …).

---

## 1. File-based logging

### `UDSACTOR_<TYPE>_LOG_PATH`

The directory (not a file) where the rolling log file lives.

| Default | Component | Notes |
| --- | --- | --- |
| `C:\Windows\Temp` (`std::env::temp_dir()`) | **Windows SERVICE / CONFIG** | Process runs as `LocalSystem`, so `TEMP` resolves to `C:\Windows\Temp`. |
| `%USERPROFILE%\AppData\Local\Temp` (`std::env::temp_dir()`) | **Windows CLIENT** | `udsactor-client.exe` is launched by the user that has logged on, so its `TEMP` is the user's temp folder. **Do not override.** |
| `/var/log/udsactor` (created if writable; falls back to `/tmp`) | **Linux / macOS SERVICE / CONFIG** | The actor tries to `mkdir -p /var/log/udsactor` and probes writability with a tiny `.udsactor-write-probe` file; if anything fails it silently falls back to `/tmp`. |
| `$TMPDIR` or `/tmp` | **Linux / macOS CLIENT** | Standard user temp. |

> If the override points to a directory that does not exist, the actor tries
> to create it (`std::fs::create_dir_all`). If creation fails, the layer
> falls back to writing into `temp_dir()/udsactor-fallback.log` and prints a
> warning to stderr.

### `UDSACTOR_<TYPE>_LOG_LEVEL`

Minimum level that gets written to the file.

| Value | Meaning |
| --- | --- |
| `trace`, `debug`, `info`, `warn`, `error` | Standard `tracing` filters. |
| `off` / `false` | Disable file logging for that component. |

Default:
- `SERVICE` → `info` (set very early in `service/main.rs`).
- `CLIENT` → `debug` (set in `client/main.rs`).
- `CONFIG` → `info`.
- `TESTS` → `debug`.

The level can be changed at runtime via `shared::log::set_log_level`.

### `UDSACTOR_<TYPE>_LOG_USE_DATETIME`

If set to `true` (case-insensitive), the log filename is augmented with the
computer name and a timestamp, e.g.
`udsactor-service-MYHOST-20260708-143012.log`. Useful when several service
instances run on the same machine (e.g. test rigs) and you want one rotated
file per start. Default: `false` → `udsactor-service.log`.

---

## 2. Log rotation

The rolling writer rotates at **16 MiB** and keeps **2 files** (the current
one and one `.log.1`). See `RotatingWriter` in `crates/shared/src/log.rs`.

Rotation is not configurable today; patching it is a localised change in that
single struct.

---

## 3. Stderr writer

A second `fmt::Layer` writes a coloured, line-numbered copy of every event
to `stderr`. This is what systemd (`StandardError=journal`), launchd
(`log show --predicate 'process == "udsactor-service"'`) and a manual
console attach all consume.

The stderr layer is **always registered** for `LogType::Service` and
`LogType::Client`; the only thing that varies is the inner `EnvFilter`
applied to it. `LogType::Config` does not get a stderr layer (it is a
one-shot GUI tool).

| Variable | Component | Default | Notes |
| --- | --- | --- | --- |
| `UDSACTOR_<TYPE>_STDERR_DISABLE` | SERVICE / CLIENT | `false` | Set to `true` / `1` / `yes` / `on` to mute stderr. The layer stays registered (so the type stays uniform across binaries); it is gated internally by `EnvFilter::new("off")`. |
| `UDSACTOR_<TYPE>_STDERR_LEVEL` | SERVICE / CLIENT | inherits `LOG_LEVEL` on release, `debug` on debug builds | Minimum level emitted to stderr. Accepted values: `trace`, `debug`, `info`, `warn`, `error`, `off`. |

> **Why "always register"?** `tracing-subscriber`'s `Layered` type changes
> every time you conditionally add a layer, which forces the
> `tracing_subscriber::registry().with(...).try_init()` chain to be
> type-specialised per branch. Keeping the layer unconditionally registered
> lets the `EnvFilter` carry the "on/off" decision and keeps the call site
> readable. The cost of an extra (filtered-out) layer is negligible.

---

## 4. Auxiliary writers (Event Viewer / syslog)

These give operators a system-level view of the actor's events without
having to open the log file.

### 4.1 Windows Event Viewer

Implemented in `crates/shared/src/windows/eventlog.rs`. It registers an
event source named `"UDS Actor Service"` against the local
`Application` log via `RegisterEventSourceW` and reports each event with
`ReportEventW`, mapping `tracing::Level` to Windows event types:
`ERROR`/`WARN` → `EVENTTYPE_WARNING` (yellow), `INFO`/`DEBUG`/`TRACE` →
`EVENTTYPE_INFORMATION` (green).

The layer is **only active** for `LogType::Service` (`EventLogLayer::for_type`
returns a no-op layer for everything else, mirroring the broker forwarder).

| Variable | Component | Default | Notes |
| --- | --- | --- | --- |
| `UDSACTOR_<TYPE>_EVENTLOG_LEVEL` | Windows SERVICE | `info` | Minimum level forwarded. Accepted values: `trace`, `debug`, `info`, `warn`, `error`, `off`. |
| `UDSACTOR_<TYPE>_EVENTLOG_DISABLE` | Windows SERVICE | `false` | Set to `true` to skip registering the event source. Useful inside CI / containers where the event-log service is absent. |

> The event source registration must happen with elevated privileges (the
> service runs as `LocalSystem`, which has them). On `EventLogLayer::shutdown`
> the handle is deregistered via `DeregisterEventSource` — called from
> `crates/service/src/main.rs` after the WS server stops.
>
> The event source entry (`UDS Actor Service`) is created with the right
> message file on install: see `building/windows/Dockerfile` and the
> installer that drops the corresponding registry keys under
> `HKLM\SYSTEM\CurrentControlSet\Services\EventLog\Application\UDS Actor Service`.

### 4.2 Syslog (Linux / macOS)

**Not implemented yet** (placeholder for parity with the v4.0 Python
actor). When added it will respect:

| Variable | Component | Default | Notes |
| --- | --- | --- | --- |
| `UDSACTOR_<TYPE>_SYSLOG_LEVEL` | Linux / macOS SERVICE | `info` | Minimum level forwarded to the local syslog (UDP `127.0.0.1:514`). Levels are mapped to RFC 3164 priorities (`daemon.notice` for `info`, `daemon.warning` for `warn`, `daemon.err` for `error`). |
| `UDSACTOR_<TYPE>_SYSLOG_DISABLE` | Linux / macOS SERVICE | `false` | Set to `true` to skip opening the syslog socket on environments where no syslogd is running. |

> On Linux distributions using `systemd-journald` instead of classic syslog,
> the same writer works because `journald` listens on `/dev/log`. On macOS
> no syslog daemon is required — the writer just opens the UDP socket.

---

## 5. Broker log forwarding (service-only)

`crates/shared/src/log_forward.rs` adds a tracing `Layer` that mirrors the
v4.0 Python actor: every `info!` / `warn!` / `error!` emitted by the service
is also pushed to the broker via `POST actor/v3/log`. This is **only**
active for `LogType::Service` — client and config never forward (the
client path already goes through the WS `LogRequest` worker instead).

The forwarder is wired by calling `shared::log_forward::set_log_forwarder(...)`
once at service startup (already done in `crates/service/src/main.rs`).
After that, each event whose level is `>= UDSACTOR_<TYPE>_FORWARD_LEVEL`
is captured, passed through a lock-free `flood_allow` (60 events / 60 s),
and `tokio::spawn`-ed to call `BrokerApi::log(...)`.

| Variable | Component | Default | Notes |
| --- | --- | --- | --- |
| `UDSACTOR_<TYPE>_FORWARD_LOGS` | **SERVICE only** | `true` | Set to `false` / `0` / `no` / `off` to disable broker forwarding for the service. Client and Config always ignore this env var. |
| `UDSACTOR_<TYPE>_FORWARD_LEVEL` | **SERVICE only** | `warn` | Minimum level forwarded. Accepted values: `trace`, `debug`, `info`, `warn`/`warning`, `error`. |

> **Why only the service?** The `broker_api` instance lives inside the
> service process. The client (`udsactor-client`) talks to the broker
> through the WS tunnel established by the service, so re-using the same
> forwarder there would just duplicate the existing
> `crates/service/src/workers/ws/logger.rs` worker.
>
> **Why not just leave the default at `info`?** INFO is the level the v4.0
> Python actor used, but in practice the Rust service emits a steady
> stream of INFO events (one per network change, per initialize attempt,
> per interface poll, …). WARN keeps the broker log meaningful and avoids
> flooding the `actor/v3/log` endpoint.

---

## 6. Quick recipes

### 6.1 Diagnose a JoinDomain failure on Windows

```powershell
$env:UDSACTOR_SERVICE_LOG_LEVEL = "debug"
$env:UDSACTOR_SERVICE_LOG_PATH = "C:\Windows\Temp"
# Restart the service
Restart-Service UDSActorService
# Watch the log
Get-Content C:\Windows\Temp\udsactor-service.log -Tail 30 -Wait
```

You should see lines like:

```
ERROR NetJoinDomain for domain 'corp.example.com', OU 'OU=Machines,…', \
  account 'svc-uds' failed: The specified network name is no longer available. \
  (code 64, 0x00000040)
```

### 6.2 Per-machine rotating logs on Linux

```bash
export UDSACTOR_SERVICE_LOG_LEVEL=info
export UDSACTOR_SERVICE_LOG_USE_DATETIME=true
sudo systemctl restart udsactor.service
ls -l /var/log/udsactor/
# -> udsactor-service-<host>-<timestamp>.log
```

### 6.3 Force the client log into a stable location (debugging)

```cmd
setx UDSACTOR_CLIENT_LOG_PATH "C:\UDS\client-logs"
setx UDSACTOR_CLIENT_LOG_LEVEL "debug"
# Re-login so the variables propagate to the user session
```

The actor also prints the resolved path to stderr at startup
(`udsactor log file: …`), so even without the env vars you can always tell
where the log is written.

### 6.4 Toggle broker forwarding or change its threshold

```powershell
# Disable forwarding entirely (still logs to file + stderr)
[Environment]::SetEnvironmentVariable("UDSACTOR_SERVICE_FORWARD_LOGS", "false", "Machine")

# Or lower the threshold to INFO (more verbose)
[Environment]::SetEnvironmentVariable("UDSACTOR_SERVICE_FORWARD_LEVEL", "info", "Machine")

Restart-Service UDSActorService
```

Useful env vars to confirm it's wired up:

| Where | What to look for |
| --- | --- |
| Service log file | First line after startup: `udsactor log file: …udsactor-service.log` |
| Broker REST log | `actor/v3/log` POSTs with the formatted message, token = the managed `own_token` |
| Server side | The broker stores these in the actor's `UserService.log` table for the admin UI |

### 6.5 Inspect the service from Event Viewer (Windows)

```powershell
# Open the most recent errors / warnings from the actor
Get-EventLog -LogName Application -Source "UDS Actor Service" -Newest 20 |
    Where-Object { $_.EntryType -in "Error","Warning" } |
    Format-List TimeGenerated, EntryType, Message

# Or use the modern cmdlet
Get-WinEvent -FilterHashtable @{LogName='Application'; ProviderName='UDS Actor Service'} -MaxEvents 20
```

To mute the EventLog writer (e.g. inside a CI container):

```powershell
[Environment]::SetEnvironmentVariable("UDSACTOR_SERVICE_EVENTLOG_DISABLE","true","Machine")
Restart-Service UDSActorService
```

To switch the EventLog writer to DEBUG while investigating:

```powershell
[Environment]::SetEnvironmentVariable("UDSACTOR_SERVICE_EVENTLOG_LEVEL","debug","Machine")
Restart-Service UDSActorService
```

### 6.6 Tail stderr alongside the file (Linux / macOS)

The systemd unit captures stderr into the journal, but during development
you may want to follow it directly:

```bash
journalctl -u udsactor.service -f
# or, when running the binary outside systemd:
RUST_LOG=debug ./udsactor-service
```

The same events show up three places (file, stderr, broker) — each layer
has its own `EnvFilter`, so you can crank one without flooding the others.

---

## 7. See also

- `crates/shared/src/log.rs` — implementation of the rolling file writer,
  the stderr layer and the layer-pipeline assembly.
- `crates/shared/src/log_forward.rs` — broker-forwarding `Layer`,
  `set_log_forwarder()` API and `flood_allow()` lock-free rate limiter.
- `crates/shared/src/windows/eventlog.rs` — `EventLogLayer`,
  `RegisterEventSourceW` + `ReportEventW` plumbing and the
  `Shutdown` integration at `crates/service/src/main.rs`.
- `crates/shared/src/windows/system/mod.rs` —
  `WindowsOperations::format_net_error`, the helper that translates Win32
  error codes (returned by `NetJoinDomain`, `NetGetJoinInformation`,
  `NetLocalGroupAddMembers`, `SetNamedSecurityInfoW`,
  `GetAdaptersAddresses`, …) into readable messages via
  `FormatMessageW(FORMAT_MESSAGE_FROM_SYSTEM)`.
- `crates/service/src/computer.rs::join_domain` — high-level orchestrator
  that calls `system.ensure_domain_membership` and propagates errors with
  `?` (no extra logging here — all diagnostic logs come from the layer below).
- `crates/service/src/workers/ws/logger.rs` — complementary worker that
  forwards **client-side** logs to the broker over the WS tunnel
  (rate-limited with its own `FloodGuard`).
