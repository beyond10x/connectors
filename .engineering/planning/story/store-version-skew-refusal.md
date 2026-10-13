---
format: aep.planning-md/3
id: story:store-version-skew-refusal
kind: story
status: draft
title: An older binary on a store a newer one wrote says so
revision: 1
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
