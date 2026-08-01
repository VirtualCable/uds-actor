# REST API Bearer migration (out of `actor_v3`)

## Scope

This note covers the **non-`actor_v3` REST endpoints** the actor
currently uses.  Those endpoints live in `crates/shared/src/broker/api/`
and are reached via `UdsBrokerApi`.  The full list of paths is:

- `GET  auth/auths` — list authenticators
- `POST auth/login` — obtain a session token
- `POST register`    — register the actor (uses the session token from
  `/auth/login` as a credential via `X-Auth-Token`)
- `POST initialize`  — initialize the actor (anonymous, token in body)
- `POST ready`       — actor is ready (anonymous, token in body)
- `POST unmanaged`   — same as `ready` for unmanaged actors
- `POST ipchange`    — IP changed notification (anonymous, token in body)
- `POST login`       — UDS-side login (anonymous, token in body)
- `POST logout`      — UDS-side logout (anonymous, token in body)
- `POST log`         — log forwarding (anonymous, token in body)

`actor_v3` itself is out of scope and **will not be migrated** before
the v6 release.

## Current authentication model (v4/v5)

- `auth/login` returns a session token.  In v4/v5, the response is a bare
  session key (no scheme tag).
- `register` is the **only** call that authenticates with a header.  The
  actor sets `X-Auth-Token: <session_key>` and the server accepts the
  bare token.
- All other `actor_v3` endpoints are anonymous on the server side; the
  actor identifies itself via a `token` field embedded in the JSON
  body of the request (the server-issued master/own token).

## Target authentication model (v6+, post-`actor_v3`)

Server-side, in v5.0 we are introducing the bearer scheme:

```
Authorization: Bearer <scheme>-<value>
```

with two known schemes:

- `ses-…` — session token, returned by `/auth/login`
- `sk-…`  — secret key, bound to a registered `Server` row (today used
  only by tunnel actors; not by the legacy REST endpoints listed
  above)

The legacy `X-Auth-Token` header will continue to work during v5/v6
(COMPAT mode) and is expected to be removed in v7 (NO_COMPAT).

## What needs to change in the actor

When we move the legacy REST endpoints listed at the top of this note
to the new model, the following touch points need to be revisited:

1. **`crates/shared/src/broker/api/block.rs`** — `register()` is the
   only call that sets `X-Auth-Token` today.  Migrate it to
   `Authorization: Bearer ses-<session_key>`.  Strip the `ses-`
   prefix the server returns from `/auth/login` if the same value is
   later reused on another endpoint that expects the bare session key
   (there is no such reuse today, so this is forward-looking only).

2. **`crates/shared/src/broker/api/mod.rs`** — `api_login()`
   currently returns the raw `response.token`.  Keep it that way for
   now (the server emits `ses-<session_key>` but the actor still passes
   it through unmodified).  When the actor itself starts using
   `Authorization: Bearer`, we will need to make sure the prefix is
   preserved on round-trip — i.e. **do not strip it** in the actor.

3. **No other actor code path uses a header credential.**  The body
   `token` field for the anonymous endpoints carries the
   server-issued master/own token, which is *not* prefixed and does
   not need to be.

## Why we are not migrating now

- `actor_v3` (the **only** endpoint family using this code path) is
  frozen until v6 at the earliest.
- The legacy `X-Auth-Token` path on the server side is fully backward
  compatible: it accepts tokens with or without the `ses-` prefix.
- The actor will keep working as-is without any code change against
  v5.0 server.

So this is a **future migration note**, not a TODO that needs to be
acted on before v5.0 ships.

## How to verify nothing has regressed in v5.0

1. Build and install v5.0 server.
2. Run the actor against it: install, authenticate, register, and
   perform at least one UDS-side login + logout.
3. Observe that the actor uses `X-Auth-Token` on `register` and that
   `register` succeeds even though the token it sends is `ses-…`-prefixed
   (the server strips the prefix before lookup).

If step 3 succeeds, v5.0 server is fully compatible with the actor as
shipped in this repo and the migration described above can be deferred
to the v6 cycle.
