# Domain membership repair on Linux (snapshot reuse scenario)

## When this matters

This document describes the **Linux** equivalent of the Windows secure-channel
repair flow described in `notes/windows-domain-trust-repair.md`. It only
matters for one very specific deployment pattern:

> A pool of machines that are **reused across user sessions via clean
> snapshots** (power off → snapshot → power on for the next user).

That is the only UDS use case in which a stale machine-account secret can end
up cached locally while the directory (AD / FreeIPA) holds a different one.
For normal dynamic desktops (cloned from a master template at deploy time,
joined fresh, never snapshotted) this flow is irrelevant — the secret is
always fresh.

This is **well under 0.1%** of UDS deployments, so the repair path is
**best-effort**: it tries to detect and fix the broken trust when it can,
and falls back to a full `realm join` (the same thing the actor already did
historically) when it cannot. Perfect coverage of every backend / distro
combination is explicitly not a goal.

## Prerequisites for the host

The detection / repair logic in
`crates/shared/src/unix/linux/computer.rs::ensure_domain_membership` relies on
real, externally-maintained CLI tools. They are **not** optional for the
repair path to work — if a tool is missing, the actor silently degrades to
"always re-join" (which still works, just heavier).

| Tool | Debian / Ubuntu | Fedora / RHEL | openSUSE | Purpose |
| --- | --- | --- | --- | --- |
| `realm` (`realmd`) | `realmd` | `realmd` | `realmd` | **Required.** Domain join/leave/list. Installed by the actor's join flow. |
| `sssd` | `sssd` | `sssd` | `sssd` | Client software used by `realmd` by default for AD/IPA. |
| `net` (`samba-common-tools`) | `samba-common-bin` / `samba-common-tools` | `samba-common-tools` | `samba-client` | **Optional but recommended.** Provides `net ads testjoin` (detect broken trust cheaply), which lets the actor skip an unnecessary re-join on healthy hosts. Only meaningful for AD memberships created with `--membership-software=samba`. |
| `adcli` | `adcli` | `adcli` | `adcli` | Alternative membership software for AD. `adcli testjoin` exists but is **not** used by the actor today (see "Limitations" below). |
| FreeIPA client (`ipa-client`) | `freeipa-client` | `ipa-client` | `ipa-client` | For IPA realms. Has no standardized CLI "is my trust alive?" probe; the actor falls back to `realm join`. |
| `krb5-user` (`kinit`/`klist`) | `krb5-user` | `krb5-workstation` | `krb5` | **Not required** by the actor. Could be used as a universal `kinit -k` trust probe, but is not installed by default everywhere, so the actor does not depend on it. |

### Recommendation

For the snapshot-reuse pattern with AD, install
**`samba-common-tools`** (provides `net`) on the master image. That lets the
actor run `net ads testjoin` to cheaply confirm the trust is healthy on every
service start and **skip** the otherwise-mandatory re-join on healthy hosts.

For IPA, no cheap probe exists in the tooling; the actor falls back to
`realm join` on every start. That is acceptable given how rare the
snapshot-reuse pattern is on IPA.

## What the actor does on each service start

```text
ensure_domain_membership(options):

  1. current = realm list --name-only   (first configured realm)
     - realm binary missing → Err  (see "Why realm is mandatory")
     - no realm configured   → realm join (full)        → Ok(true)
     - current != requested  → realm join (full)        → Ok(true)

  2. We are in the requested realm. Is the trust alive?
     - if `net` is installed:
         net ads testjoin
           OK   → done, trust healthy (no re-join)      → Ok(false)
           FAIL → realm join (full)                     → Ok(true)
     - else (no `net`, e.g. IPA-only host):
         → realm join (full, best effort)               → Ok(true)

  There is no credential-less repair on Linux for a stale machine secret
  (see "Why repair always means realm join" below), so a failed testjoin
  always falls back to `realm join`. The only benefit of `net ads testjoin`
  is to SKIP the re-join on healthy hosts.

  `realm join` restarts sssd but does NOT reboot the OS. We still return
  Ok(true) when we changed domain state, to stay honest with the
  System::ensure_domain_membership contract ("did we change domain state?").
```

## Why repair always means `realm join`

On Windows, repairing a broken secure channel can be done with
`NetJoinDomain` using admin credentials (Path A in
`notes/windows-domain-trust-repair.md`). The Linux equivalent is `realm join`,
again using admin credentials.

You might expect `net ads changetrustpw` to be a lighter repair, but it is
**not** usable for the snapshot-stale case: `changetrustpw` rotates the
machine secret by **authenticating with the current local secret**. In the
post-snapshot state the local secret is stale (it does not match what the DC
holds), so the authentication step fails and `changetrustpw` fails too. The
same is true of any credential-less rotation. Therefore the only repair that
works is a full re-join with directory-admin credentials from
`JoinDomainOptions`.

The value of probing at all is purely to **avoid** the re-join (and its sssd
restart) on hosts whose trust is still healthy.

## Why `realm` is mandatory

The actor joins domains **exclusively** through `realm join` (see
`crates/shared/src/unix/linux/computer.rs::join_domain`). Therefore, if a
host reports being in a domain, `realm` was installed at join time and must
still be present. A host with no `realm` binary cannot have been joined by
us, so `get_domain_name` returns an **error** (not `None`) in that case —
returning `None` would be a lie and would make `ensure_domain_membership`
re-join every boot.

## Commands actually used by the actor (verified, not invented)

All of the following have been checked against the installed manpages
(`realm(8)`, `net(8)`) and the binaries' own `--help` output. There is no
`realm check` and no `realm refresh` — those are commonly-cited but
**do not exist**.

| Command | Manpage wording | Used for |
| --- | --- | --- |
| `realm list --name-only` | "List all the discovered and configured realms" | Detect which realm we are joined to. |
| `realm join [flags] REALM` | "Configure the local machine for use with a realm" | Full (re)join / repair. Flags used by the actor: `--user=`, `--client-software=`, `--server-software=`, `--membership-software=`, `--computer-ou=`, `--use-ldaps`, `--automatic-id-mapping=no`. |
| `net ads testjoin` | "Check whether participation in a domain is still valid" | Trust-health probe (samba membership only). |

## Limitations (accepted, best-effort)

- **IPA** has no cheap, universal "is my host keytab still valid?" probe that
  the actor can rely on without extra packages. On IPA the actor always
  re-joins. Acceptable because the snapshot-reuse pattern is rare on IPA.
- **adcli memberships** (`net ads testjoin` does not apply) are not probed
  today; the actor re-joins. `adcli testjoin` exists but verifying its exit
  semantics across versions is deferred.
- **`kinit -k`** from `/etc/krb5.keytab` would be the only truly universal
  probe (works for both AD and IPA), but it requires `krb5-user`, which is
  not installed by default on minimal images. We do not depend on it.
- **`sssctl domain-status`** reports SSSD's view of the domain, not the
  machine-account secret validity, so it is not a reliable trust probe and
  is intentionally not used.

## Forcing a broken trust for testing

On the directory side, reset (do not delete) the computer account:

- **AD**: `Reset-ADComputer -Identity <host>` in PowerShell, or
  `adsiedit.msc` → right-click → Reset Account.
- **IPA**: `ipa host-disable <host>` (then re-enable on the server after the
  client tries).

Then on the client, before the actor runs, confirm the breakage:

```bash
sudo net ads testjoin           # should now FAIL (samba)
sudo realm list --name-only     # still shows the realm (looks joined!)
```

This reproduces exactly the post-snapshot state: `realm list` still reports
the realm (the local config is intact), but the trust is dead.
