---
title: The local runtime
sidebar_position: 4
description: How the local connectors CLI serves configured instances, keeps credentials in custody, records metadata and runs approved writes.
lede: The local CLI never holds a credential in a file or a command line; a local owner starts the exact process that serves each instance, the keyring holds credentials and Entity Runtime records everything else.
source: contracts/cli/v1alpha1/semantics.md, contracts/auth/connection/v1alpha1/semantics.md, crates/connectors-host/src/local/config.rs, crates/connectors-host/src/local/registry/stale_tests.rs, docs/local-er-metadata.md, docs/local-mounted-secret-custody.md, docs/local-approvals.md, docs/local-clock.md, docs/local-consumer-launch.md, apps/connectors/tests/local_cli.rs
---

# The local runtime

The grouped `connectors` commands (`setup`, `instances`, `connections`, `operations` and
`approvals`) run against three local parts. The terms connector, instance and connection are
those of the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary).

| Part | What it holds | Where |
|---|---|---|
| Configuration | the enabled connectors and their instances, each instance with an optional endpoint and description; consumers; the approval clock | a private TOML file, owner-only |
| Metadata store | connections and their identities and states, validation evidence, approval policy and keys, mutation attempts and execution audit | Entity Runtime records over an Eventlog SQLite database |
| Custody | credentials | the platform's protected store, fixed when the binary is built: the Secret Service keyring on Linux, in a qualified binding, the login keychain on macOS and the Credential Manager on Windows. On a headless or container host, the owner configuration can instead select a read-only mounted directory (`mounted_secret`), never as a fallback |

A **local owner** process starts on demand, holds the metadata store and starts the process that
serves each instance as a child over a private protocol. It starts only the exact executable the
configuration pins or derives, refuses a changed executable or configuration before any provider
work, and stops exact incarnations. A CLI built differently from the running owner is refused
rather than served. [Set up the local CLI](../guides/set-up-the-local-cli.md) creates the
configuration and the store.

## Saved connections

Each instance holds at most one connection. `connections connect --connector <connector>
--instance <instance> --profile <profile>` runs the named auth profile's identity probe (and,
where the profile declares them, its scope and access reads) and saves the credential in custody. The credential enters
through a hidden prompt or a protected file, never as an argument. A connect on an instance that
already holds a Live or Stale connection is refused before any credential entry, as
`lifecycle_conflict` with the next action that helps: reconnect, revalidate or revoke. A second
account is a second instance.

| State | Meaning | Leaves it by |
|---|---|---|
| `Live` | no refusal of the stored credential is recorded; whether it is ready is a separate check | the provider refusing it (`Stale`), or `connections revoke` (`Revoked`) |
| `Stale` | the provider refused the stored credential: an expired or revoked refresh token, a changed password, a revoked token. No retry can succeed | `connections reconnect` publishing a new credential (`Live`), or `connections revoke` |
| `Revoked` | local use ended | nothing; it stays as history |

Refreshing an OAuth access token is not a state change; only a refused refresh token makes a
connection Stale. Then:

- `connections revalidate` renews the validation evidence without asking for the credential
  again. Evidence lasts 60 seconds; an invoke on lapsed evidence answers with the next action
  `revalidate_connection`.
- `connections reconnect` replaces the credential of the instance's connection, keeps its
  reference and refuses a credential that names a different identity. An invoke on a Stale
  connection answers with the next action `repair_connection`, which names this command.
- `connections revoke` ends local use. It does not revoke the token at the provider.
- `connections launch` hands one connection's credential to a program pinned in the
  configuration, on file descriptor 3, and to nothing else.

`connections describe`, `reconnect`, `revalidate`, `launch` and `revoke` select the instance's
one connection when `--connection` is left out. A revalidation whose answer never arrives is
`outcome_unknown`, not a failure.

## Approved writes

A write runs through private protocol two, under the same rules for every provider:

1. `approvals policy-set` publishes which write operations an instance may be asked for.
2. `approvals prepare` resolves the exact subject of one write (connection, operation, schema,
   descriptor revision and whole input) and its SHA-256, without reading a credential.
3. `approvals issue` signs a protected proof for that subject with a local approval-signing key.
4. `operations invoke` presents the proof. The host records the attempt before its one dispatch,
   spends the approval once and records the outcome in the execution audit.

Approval-signing keys are created, rotated, recovered, revoked and retired with
`approvals key-init`, `key-rotate`, `key-recover`, `key-revoke` and `key-retire`. Issuing a proof
also requires a configured bounded clock; the
[clock guide](https://github.com/beyond10x/connectors/blob/main/docs/local-clock.md) configures
its sources and `approvals clock-check` checks them.
Reads need no approval and no clock.

The [CLI contract](../reference/contracts/cli.md) states every rule of these commands, and the
[CLI reference](../reference/cli.md) lists their options.
