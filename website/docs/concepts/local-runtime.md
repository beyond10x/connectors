---
title: The local runtime
sidebar_position: 4
description: How the local connectors CLI supervises adapters, keeps credentials in custody, records metadata and runs approved writes.
lede: The local CLI never holds a credential in a file or a command line; a local owner supervises exact adapter processes, the keyring holds credentials and Entity Runtime records everything else.
source: contracts/cli/v1alpha1/semantics.md, docs/local-er-metadata.md, docs/local-mounted-secret-custody.md, docs/local-approvals.md, docs/local-clock.md, docs/local-consumer-launch.md, apps/connectors/tests/local_cli.rs
---

# The local runtime

The grouped `connectors` commands (`setup`, `adapters`, `connections`, `operations` and
`approvals`) run against three local parts:

| Part | What it holds | Where |
|---|---|---|
| Configuration | the configured adapters, each with its executable's path and SHA-256 and the configuration revision it serves; consumers; the approval clock | a private TOML file, owner-only |
| Metadata store | connections and their identities, validation evidence, approval policy and keys, mutation attempts and execution audit | Entity Runtime records over an Eventlog SQLite database |
| Custody | credentials | the platform's protected store, fixed when the binary is built: the Linux Secret Service keyring, in a qualified binding. macOS login keychain and Windows Credential Manager bindings exist; the CLI does not yet run on macOS or Windows. On a headless or container host, the owner configuration can instead select a read-only mounted directory (`mounted_secret`), never as a fallback |

A **local owner** process starts on demand, holds the metadata store and supervises the adapter
executables as child processes over a private protocol. It starts only the exact executable the
configuration pins, refuses a changed executable or configuration before any provider work, and
stops exact incarnations. A CLI built differently from the running owner is refused rather than
served. [Set up the local CLI](../guides/set-up-the-local-cli.md) creates the configuration and
the store.

## Saved connections

`connections connect` runs the adapter's identity probe (and, where the profile declares them,
its scope and access reads) and saves the credential in custody. The credential enters through a
hidden prompt or a protected file, never as an argument. Then:

- `connections revalidate` renews the validation evidence without asking for the credential
  again. Evidence lasts 60 seconds by default; a read on lapsed evidence renews it once the same
  way before it is served, so a connection keeps serving reads with no caller revalidation while
  its credential stays valid. A consumer launch does not, and answers `unavailable`.
- `connections repair` replaces an invalid credential and refuses one that names a different
  identity.
- `connections revoke` ends local use. It does not revoke the token at the provider.
- After the adapter's configuration revision changes, a connection follows it on its next
  revalidation when its provider authority, profile and identity are unchanged; otherwise the
  answer names `create_connection`.
- `connections launch` hands one connection's credential to a program pinned in the
  configuration, on file descriptor 3, and to nothing else.

A revalidation whose answer never arrives is `outcome_unknown`, not a failure.

## Approved writes

A write runs through private protocol two, under the same rules for every provider:

1. `approvals policy-set` publishes which write operations an adapter may be asked for.
2. `approvals prepare` resolves the exact subject of one write (connection, operation, schema,
   descriptor revision and whole input) and its SHA-256, without reading a credential.
3. `approvals issue` signs a protected proof for that subject with a local approval-signing key.
4. `operations invoke` presents the proof. The host records the attempt before its one dispatch,
   spends the approval once and records the outcome in the execution audit.

Approval-signing keys are created, rotated, recovered, revoked and retired with
`approvals key-init`, `key-rotate`, `key-recover`, `key-revoke` and `key-retire`. Issuance needs a bounded clock: `approvals clock-check` asks two to
eight explicitly configured Roughtime sources in parallel and answers only where a `quorum` of
them agree, so one lying source cannot move the answer off true time and one unreachable source
does not stop it. It reports each source's result, uses the operator's stated bound on the local
timer's rate error, and starts no owner or adapter. This configuration uses two of the public
sources checked on 2026-10-14 (the [clock guide](https://github.com/beyond10x/connectors/blob/main/docs/local-clock.md)
lists all four) and an assumed 1% rate bound, which is a stated assumption, not a calibration:

```toml
[approval_clock]
format = "roughtime-quorum/1"
quorum = 2
max_rate_error_ppm = 10000

[[approval_clock.sources]]   # int08h, roughtime.int08h.com
address = "51.81.57.55:2002"
public_key = "AW5uAoTSTDfG5NfY1bTh08GUnOqlRb+HVhbJ3ODJvsE="
version = "roughtime-0x8000000c"

[[approval_clock.sources]]   # time.txryan.com
address = "148.113.255.71:2002"
public_key = "iBVjxg/1j7y1+kQUTBYdTabxCppesU/07D4PMDJk2WA="
version = "roughtime-0x8000000c"
```

These two were the only sources that answered on 2026-10-14, so this is a quorum of two of two:
it answers, but tolerates no outage and no lying source until a third source answers. A verified
source counts at any radius (int08h signed 5 s, time.txryan.com 3 s); only the combined answer
is capped at eight seconds. `roughtime-0x8000000c` is the one admitted version; any other is
refused as unknown.

The single-source `roughtime-clock/1` table is retired: it still loads, but every use of the
clock refuses it with `configuration_rule` `approval_clock_format_retired` until it is rewritten
in this shape.

Reads need no approval and no clock.

## Cost of the metadata store

Every fresh open of the store verifies all of it, so a raw edit that bypasses SQLite is refused by
the next command. While an owner runs, a command reads the store through the owner's handle, and
the per-invoke cost no longer grows with the number of recorded events (since 0.33.0). Stores
created by `setup init` carry durable open checkpoints; an older store gets them with
`setup checkpoints-enable`, which is one-way: connectors 0.32.0 and earlier cannot open it
afterwards.

The [CLI contract](../reference/contracts/cli.md) states every rule of these commands, and the
[CLI reference](../reference/cli.md) lists their options.
