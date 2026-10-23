---
title: Upgrade the CLI
sidebar_position: 6
description: What a new connectors binary does with a state directory an older or newer one wrote, and how to stop the owner the previous binary started.
lede: A state directory is admitted only to a binary at least as new as its newest writer; after an upgrade, setup owner-stop retires the owner the old binary started.
source: apps/connectors/tests/store_version_skew.rs, apps/connectors/tests/local_cli.rs, crates/connectors-host/src/local/metadata.rs, contracts/cli/v1alpha1/owner.md, contracts/cli/v1alpha1/semantics.md
---

# Upgrade the CLI

Read this before you replace the `connectors` binary, or when two binaries of different versions
use one machine. The commands run from `target/try` with the configuration and state directory
that [Set up the local CLI](./set-up-the-local-cli.md) creates.

## Binaries of different versions and one state directory

The metadata store records the newest binary that wrote it and a digest of the record model that
binary persists. Once a binary has written a state directory, an older binary may not use it:
binaries of different versions share one state directory only while the oldest of them is at
least the newest that wrote it. An older binary whose record model differs refuses the store
before it reads anything, in every command and `setup check` included: exit status 1, failure
code `metadata_schema_newer` at stage `admission` with `next_action` `upgrade`, and the failure
names `store_version` (the newest writer's release) and `binary_version` (its own). Retrying does
not change that answer; upgrading the binary does. A newer release that changed no
persisted record is admitted. When the newest writer cannot be named, because the store was
migrated to a level this binary does not know or its recorded version is not one this binary can
order, the refusal is the same with `store_version` `unrecorded`. Releases up to 0.49.0 cannot read the marker: on a store a newer
binary wrote they answer `metadata_unavailable` with `next_action` `retry_status` instead, and
retrying does not help them either.

A consumer pinned to one version keeps its own state directory. `setup init` with the existing
configuration and a new `--state-dir` initialises an empty store there and leaves the
configuration unchanged:

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/pinned/state" \
  --output json setup init | jq '{ok, disposition: .result.disposition}'
```

```json
{
  "ok": true,
  "disposition": "state_initialized"
}
```

It then connects in that state directory, passing the same `--state-dir` to `connections connect`
and to every later command. A connection is recorded in the
state directory it was made in: connections, list cursors, approvals and cached instance
descriptions in another state directory do not carry over, so the pinned consumer connects once
more there.

## After an upgrade

The owner is the background `connectors` process that serves a state directory; a connect or an
invoke starts it. An owner the previous binary started keeps running after an upgrade, and every
command that needs it is refused with code `owner_build_mismatch` at stage `readiness`, with
`next_action` `stop_owner` and exit status 1
(`apps/connectors/tests/local_cli.rs`, `a_cli_refuses_an_owner_running_a_different_build_and_leaves_it_running`).

`setup owner-stop` asks that owner, whatever binary started it, to drain and exit, and waits up
to 30 seconds for it. Here no owner runs, since nothing has connected:

```sh
../debug/connectors --config "$PWD/cli/config.toml" --state-dir "$PWD/cli/state" \
  --output json setup owner-stop | jq '{ok, disposition: .result.disposition}'
```

```json
{
  "ok": true,
  "disposition": "no_owner"
}
```

With no owner running it answers `no_owner`, exits 0 and never starts one. With one, it answers
`"disposition": "stopped"` and names the `host_incarnation` that exited; the next command that
needs an owner starts one from the current binary. It sends the owner's own shutdown request over
the state directory's socket and never signals a process. An owner that holds
the state directory but does not answer within the 30 seconds is refused as `unavailable`; one
that is still running at the bound is refused as `timeout`; and a state directory, socket or lock
file of another user is refused as `forbidden`, each at stage `stop`.
