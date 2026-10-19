---
format: aep.planning-md/3
id: story:definition-connect-begin-carries-selection
kind: story
status: draft
title: A definition connect's Begin carries the selection its question was asked for
summary: closes the window left after 439ed1a07 between the entry re-check and the owner Begin
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

A `--definition` connect's owner `Begin` carries the selection the CLI checked when it asked the
connect question; the owner refuses `Begin` with `revision_conflict` when the instance entry's
selection differs.

## Why now

Release review run 2 (lane s2 F2, 2026-10-19) found that a run replacing the instance entry between
the connect question and `Begin` connects under the other file's host. Commit 439ed1a07 made
`connect()` re-check the entry before `admit_capture`, which closes most of the gap; a window of a few
milliseconds remains between that re-check and `Begin`, which does not carry the expected selection
(`apps/connectors/src/local/definition.rs:1067-1077`, `crates/connectors-host/src/local/owner/transport.rs:415`).

## Acceptance

- The owner protocol's `Begin` carries the expected selection digest; the owner compares it with the
  entry it loads and refuses a mismatch as `revision_conflict`.
- A test replaces the instance entry after the CLI's check and before `Begin` (hook in the test
  harness) and asserts the refusal and that no credential was sent.
