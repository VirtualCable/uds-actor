# Repairing a broken AD secure channel on Windows

## The problem

After restoring a VM snapshot (or any image-based backup), the local copy of
the machine account password stored in the LSA may not match the password the
Domain Controller (DC) holds for that computer account. The mismatch breaks the
**secure channel** between the workstation and the domain.

Typical lifecycle that triggers this:

1. The actor joins the machine to the domain (a new machine account secret is
   negotiated with the DC and cached locally).
2. The machine is powered off and a snapshot is taken.
3. Time passes; the DC rotates / expires the machine account secret (default
   `MaximumPasswordAge` is 30 days), or an admin resets the computer account.
4. The snapshot is restored and the machine boots.
5. The local LSA still holds the **old** machine secret. Authentication to the
   domain fails with `STATUS_TRUSTED_DOMAIN_FAILURE (0xC0000186)` /
   `ERROR_TRUST_FAILURE (0x6FC / 1788)`. Users cannot log on interactively
   with domain credentials.

Note that `NetGetJoinInformation` **still reports the machine as joined** to the
right domain in this situation, so a naive "am I joined?" check does not detect
the broken trust. The secure channel itself has to be probed.

## Detection: `I_NetLogonControl2` with `NETLOGON_CONTROL_TC_QUERY`

The Netlogon service exposes a query-only function code that returns the current
state of the secure channel to the trusted domain:

```c
NET_API_STATUS I_NetLogonControl2(
    LPCWSTR ServerName,     // NULL = local machine
    DWORD   FunctionCode,   // NETLOGON_CONTROL_TC_QUERY = 6
    DWORD   QueryLevel,     // 1 = return NETLOGON_INFO_1
    LPBYTE  Data,           // pointer to LPWSTR with the trusted domain name
    LPBYTE  *Buffer         // out: NETLOGON_INFO_1, free with NetApiBufferFree
);
```

`Data` for `TC_QUERY` / `REDISCOVER` is a `LPWSTR*` (pointer to a pointer to a
NUL-terminated UTF-16 string) containing the **trusted domain name**. Passing
`NULL` here returns `ERROR_INVALID_PARAMETER` on modern Windows.

The returned `NETLOGON_INFO_1` layout is:

```c
typedef struct _NETLOGON_INFO_1 {
    DWORD          netlog1_flags;                // offset 0
    NET_API_STATUS netlog1_pdc_connection_status;// offset 4
} NETLOGON_INFO_1;
```

- `netlog1_flags` carries informational flags (`NETLOGON_HAS_IP`,
  `NETLOGON_HAS_TIMESERV`, `NETLOGON_DNS_UPDATE_FAILURE`, ...). The
  `NETLOGON_VERIFY_STATUS_RETURNED (0x80)` constant belongs to
  `NETLOGON_INFO_3.netlog3_flags`, **not** to `NETLOGON_INFO_1`, so it must not
  be tested against `netlog1_flags`.
- `netlog1_pdc_connection_status` is the **authoritative** trust-health field:
  - `0`  → secure channel is healthy
  - non-zero → the Win32 error code of the last failed secure-channel operation
    (e.g. `ERROR_TRUST_FAILURE = 1788`, `ERROR_NO_SUCH_DOMAIN = 1355`,
    `ERROR_NO_LOGON_SERVERS = 1311`).

This mirrors what `nltest /sc_query:<domain>` prints on the command line.

## Path A (current implementation): re-join with `NetJoinDomain` (reboot required)

When the secure channel is broken we delegate the repair to `NetJoinDomain`
with the flags `NETSETUP_DOMAIN_JOIN_IF_JOINED | NETSETUP_JOIN_DOMAIN` (and
optionally `NETSETUP_ACCT_CREATE`), passing the domain administrator credentials
supplied by the broker in `JoinDomainOptions`.

Why this works for our scenario:

- The computer account **still exists** in the AD (it was reset / its password
  expired, but the account object was not deleted). `NETSETUP_DOMAIN_JOIN_IF_JOINED`
  tells NetJoinDomain to reuse the existing account instead of trying to create
  a new one.
- `NETSETUP_JOIN_DOMAIN` re-negotiates the machine account secret against a DC,
  authenticating the operation with the supplied domain credentials. After the
  call, the DC's copy of the machine secret and the LSA's local copy match
  again.
- `NETSETUP_ACCT_CREATE` additionally allows the account to be re-created if it
  has been deleted, which is useful when the snapshot is very old.

Trade-off: per the official `NetJoinDomain` documentation:

> A system reboot is required after calling the NetJoinDomain function for the
> operation to complete.

So Path A always returns `Ok(true)` ("reboot needed"). The caller (managed
actor) reboot the machine, after which the secure channel is restored.

This is the same effect as `nltest /sc_reset:<domain>` followed by a reboot, or
`netdom reset <machine> /domain:<domain>` followed by a reboot. It is reliable
and uses only public Win32 APIs that the `windows` crate exposes.

## Path B (future, no-reboot): `NetrServerPasswordSet2` via MS-NRPC

`netdom resetpwd /Server:<dc> /UserD:<user> /PasswordD:<pwd>` resets the machine
account secret **without a reboot**. Internally it does **not** call
`NetJoinDomain`; it speaks the Netlogon RPC protocol (MS-NRPC) directly to a DC
and invokes the `NetrServerPasswordSet2` (opnum 30) method, which atomically
sets a new machine account password on the DC. Because no local join state
changes, no reboot is required.

References:
- [MS-NRPC](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-nrpc)
  — Netlogon Remote Protocol specification.
- `NetrServerPasswordSet2` (section 3.5.4.4.1 of MS-NRPC): the RPC method
  itself, with the `NETLOGON_PASSWORD_NEW` plaintext / encrypted payload.
- `nltest /sc_resetpwd` and `netdom resetpwd` both wrap this call.

### Why this is a lot more work

1. **`NetrServerPasswordSet2` is not in the `windows` crate.** It is part of the
   MS-NRPC wire protocol, not of the Win32 API surface. The `windows` crate only
   publishes `I_NetLogonControl2` (a thin local wrapper around the Netlogon
   service), not the remote RPC methods. There is no `windows::Win32::*` binding
   for `NetrServerPasswordSet2`, `NetrServerReqChallenge`, `NetrServerAuthenticate3`,
   etc.

2. **A full MS-NRPC exchange is required.** Just calling
   `NetrServerPasswordSet2` over a raw RPC binding is not enough. The client and
   DC must first perform a Netlogon credential challenge / response:

   1. `NetrServerReqChallenge` — exchange random nonces (ClientChallenge /
      ServerChallenge).
   2. Compute the session key from the machine account password (NT hash) and
      the two challenges, using the Netlogon Credential computation
      (`NetrServerAuthenticate3`).
   3. `NetrServerAuthenticate3` — prove knowledge of the machine account secret
      and negotiate the session key + negotiated flags (AES vs RC4, etc.).
   4. `NetrServerPasswordSet2` — sealed/signed with the session key, sets a new
      `NETLOGON_PASSWORD_NEW` payload containing the new plaintext password and
      the `ClearPasswordLength` / `Encryption`/`Signature` fields per
      `[MS-NRPC] 2.2.1.3.13` and `2.2.1.4.4`.

   All four steps involve manually-constructed RPC stubs.

3. **Encryption.** The `NetlogonPasswordEncrypt` algorithm (RC4-DES-CRC for
   legacy, AES-128-CFB8 + HMAC-SHA256 for `NETLOGON_NEG_SUPPORTS_AES`) must be
   implemented in Rust to seal the new password before sending it on the wire.
   Both are specified bit-for-bit in MS-NRPC and must be byte-exact.

4. **RPC binding.** A `ncacn_ip_tcp` (or `ncacn_np`) binding to the DC's
   Netlogon endpoint has to be created. This is doable with the
   `windows` crate's `RPC_*` bindings, but it is verbose (security provider
   `RPC_C_AUTHN_GSS_NEGOTIATE` / `RPC_C_AUTHN_LEVEL_PKT_PRIVACY`, endpoint
   resolution, etc.).

5. **There is no publicly supported Rust wrapper** for this. The most realistic
   options are:

   - **Hand-roll the RPC stubs** using the `windows` crate's
     `RPC_*` / `MIDL_*` primitives. Significant effort: ~500-800 LOC of unsafe
     FFI plus the crypto. Maintenance burden when the crate evolves.
   - **Shell out to `nltest /sc_resetpwd:<domain>\<dc>`** (or `netdom resetpwd`)
     via `Command::new(...)`. Trivial (~20 LOC) but introduces a runtime
     dependency on RSAT tools (`nltest.exe` ships in-box on Windows 10+, but
     `netdom.exe` requires the RSAT feature to be enabled).
   - **Reuse an existing Rust MS-NRPC implementation** if one becomes available
     (e.g. the `ntlm` / `kerberos` crates in the `ldap3` ecosystem). At the time
     of writing (2026) there is no production-grade Rust crate that exposes
     `NetrServerPasswordSet2`.

### Recommended next step for Path B

Three viable approaches, in increasing order of effort:

1. **Shell out to `nltest /sc_resetpwd:<domain>`** (recommended).
   `nltest.exe` ships in-box on every supported Windows release
   (Windows 10 / 11, Server 2012 R2 → 2025) and is effectively a thin wrapper
   around `NetrServerPasswordSet2`. Trivial to implement (~20 LOC):

   ```rust
   let status = Command::new("nltest.exe")
       .args(["/sc_resetpwd", &options.domain])
       .status()?;
   if status.success() { Ok(false) } else { /* fallback to Path A */ }
   ```

   The downside is spawning an external process and parsing its exit code, but
   it is by far the best effort/value trade-off for a no-reboot path and the
   only dependency is a tool that is already in the OS image.

2. **Shell out to `netdom resetpwd /Server:<dc> /UserD:<user> /PasswordD:<pwd>`**.
   This is exactly the command the cievo.sk article uses, but `netdom.exe`
   requires the RSAT feature to be enabled, so it is a heavier dependency than
   `nltest.exe`. Otherwise equivalent to option 1.

3. **Hand-roll the MS-NRPC exchange in Rust** as described above. This removes
   any external dependency but is the most expensive option by a wide margin
   (~500-800 LOC of unsafe FFI + crypto), so it should only be considered if
   options 1 and 2 turn out to be unreliable on some target Windows version.

## Summary of the implemented behaviour (Path A)

`crates/shared/src/windows/system/mod.rs::ensure_domain_membership`:

1. If the machine is not joined, or is joined to a different domain than
   requested → `join_domain` (full join) and return `Ok(true)` (reboot).
2. Otherwise probe the secure channel with
   `I_NetLogonControl2(NETLOGON_CONTROL_TC_QUERY, QueryLevel=1, Data=<domain>)`.
   Read `netlog1_pdc_connection_status` (the second `u32` of `NETLOGON_INFO_1`).
3. If the connection status is `0`, the secure channel is healthy → `Ok(false)`
   (nothing to do, no reboot).
4. If broken → `rejoin_preserving_account` (`NetJoinDomain` with
   `NETSETUP_DOMAIN_JOIN_IF_JOINED | NETSETUP_JOIN_DOMAIN`, optionally
   `NETSETUP_ACCT_CREATE`) and return `Ok(true)` (reboot required).
5. The caller (managed actor) performs the reboot; on next boot the secure
   channel is restored.

## References

- `I_NetLogonControl2` — https://learn.microsoft.com/en-us/windows/win32/api/lmaccess/nf-lmaccess-i_netlogoncontrol2
- `NETLOGON_INFO_1` — https://learn.microsoft.com/en-us/windows/win32/api/lmaccess/ns-lmaccess-netlogon_info_1
- `NetJoinDomain` — https://learn.microsoft.com/en-us/windows/win32/api/lmjoin/nf-lmjoin-netjoindomain
- `netdom reset` — https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/netdom-reset
- `netdom resetpwd` — https://learn.microsoft.com/en-us/windows-server/administration/windows-commands/netdom-resetpwd
- `MS-NRPC` (Netlogon Remote Protocol) — https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-nrpc
- Practical write-up of the exact failure mode addressed here:
  https://www.cievo.sk/2012/02/21/reset-computer-accounts-in-active-directory-domain/
