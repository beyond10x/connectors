---
format: aep.planning-md/3
id: story:store-version-skew-refusal
kind: story
status: active
title: An older binary on a store a newer one wrote says so
relations:
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T21:20:40Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"verification":1}}}
- {from: "proposed", to: "active", at: "2026-10-13T21:21:20Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"verification":1}}}
---
## Problem

Observed 2026-10-13 by a scheduled consumer on its shared state directory (`~/.local/state/connectors`):
connectors 0.33.0 `connections list --adapter tavily --output json` answered `metadata_unavailable`
at stage `observation` with `next_action: retry_status`, exit 1, while 0.45.0 listed the
connection. The consumer retried an error that retrying could not fix for two days, then disabled
its source.

What is known and what is not:

- Verified by the reporter: 0.33.0 `setup check` reports metadata `ready` on the same store; only
  `connections list` fails, at observation. 0.47.0 on the live store: `setup check` and the list
  pass. `schema_migrations` holds 1 and 9.
- A copy of the state is not a test bed: 0.45.0 also fails on a copy.
- Not known: the cause. No mechanism is named until the failure is reproduced here.

## Outcome

- An older binary on a store a newer binary wrote answers a distinct, non-retryable refusal (a
  code such as `metadata_schema_newer`) that names the store's version and the binary's version,
  with `next_action` upgrade, never `retry_status`.
- The documentation states whether binaries of different versions may share one state
  directory, and how a pinned consumer keeps its own state (`--state-dir`) with an existing
  connection.

## Order

1. Reproduce: build the failing pair from tags, on a store produced the way the live one was,
   and observe which read fails at observation and why. Record the trace as evidence.
2. Spec first: the refusal code and its outcome in `ess/domains/cli.yaml` and the contract, then
   the implementation and a test per binary pair.

## Acceptance

- A test opens a store whose recorded version is newer than the binary's and gets the new code,
  both versions named, `next_action` upgrade, exit status non-zero.
- The CLI guide's state-directory section answers the sharing question and shows `--state-dir`
  for a pinned consumer, with the commands run in the tree.

## Findings

Reproduced 2026-10-13 on fresh state directories with fixture connections and a disposable
keyring; no provider calls, the consumer's store untouched.

- **Verified.** 0.33.0 answers `metadata_unavailable` (`observation`, `retry_status`, exit 1) on
  every `connections list` and `connections describe` once a 0.45.0 or 0.49.0 owner has launched
  an adapter on the store (connect or revalidate), in every order tried; 0.33.0 `setup check`
  stays ready; the newer binary lists the connection every time. Newer `setup check`, `list` and
  `describe` alone do not trigger it.
- **Verified.** What the newer binary wrote: the Entity Runtime record field
  `connectors.cli.LocalRuntimeRecord.fields.build_version` (in `connectors_er_blobs`). 0.33.0
  re-records `RememberLocalRuntimeBootstrap` without it, and the write is rejected with
  `fields.build_version: field is not declared in the schema`.
- **Inferred from the v0.33.0 source.** `Registry::list` persists after its transaction; the
  runtime record compares as changed, 0.33.0 renders it without the field, the batch fails and
  `map_execution_failure` and `host_failure` turn it into `MetadataUnavailable`, which the CLI
  maps to `observation`/`retry_status`. `setup check` never persists, so it stays ready; the
  difference is permanent, so retrying never helps.
- **Inferred.** The first writer that breaks 0.33.0 is 0.36.0 (`build_version` first appears in
  `27820aa0e`). Any field an older reader's projection drops breaks it the same way
  (`refused_configuration_revision`, 0.34.0, is another).
- **Verified.** Nothing on the store can signal the skew today: `user_version` 9,
  `schema_migrations` 1 and 9, `source_level`/`projection_level` 1/3, the Entity Runtime event
  schema versions and `definition_version` 1 are identical on stores 0.33.0, 0.45.0 and 0.49.0
  wrote.
- **Verified.** A whole-directory `cp -a` copy works with 0.45.0; nothing binds the state to its
  path or inode. A copy without `registry-clock.floor` fails `list` within 60 s of the last clock
  record; a copy without the `-wal` file fails `setup check`.

## Design

Binaries already released cannot be changed: 0.33.0 to the release before this one keep
answering `metadata_unavailable` on a store a newer binary wrote. This story makes every binary
from its release on refuse by name, and documents the rule for the binaries before it.

- **A store-level writer marker.** The metadata store records the newest writer it has seen: the
  writing binary's version and a digest of the record model it persists (the Entity Runtime
  schema the store's records are checked against). A binary raises the marker in the transaction
  of its first write when its own version is newer or the marker is absent; it never lowers it.
- **The check.** Every command that opens the metadata store, `setup check` included, reads the
  marker before its first read of records. When the marker's model digest differs from the
  binary's own and the marker's version is newer than the binary's, the command refuses with
  `metadata_schema_newer`, exit 1, `next_action = upgrade`, and the failure names
  `store_version` (the marker's) and `binary_version` (its own). The same model digest under a
  newer version (a release that changed no persisted record) is admitted.
- **Defence in depth.** A store write the Entity Runtime refuses because a record carries a field
  the binary's schema does not declare is also `metadata_schema_newer`, with `store_version`
  `unrecorded` when no marker names it, never `metadata_unavailable` with `retry_status`.
- **Sharing a state directory.** The CLI guide's state-directory section states the rule: once a
  binary has written a state directory, an older binary may not use it; binaries of different
  versions share one only while the oldest is at least the newest that wrote it. A consumer
  pinned to a version keeps its own state directory (`--state-dir`) and connects there; the
  section shows the commands, run in the tree, and says what a connection in another state
  directory does not carry over.

Spec first: the code, its `next_action`, its two fields and the marker go into
`ess/domains/cli.yaml` and `contracts/cli/v1alpha1/semantics.md` (the refusal table and the
metadata section), regenerated, before any host change.
