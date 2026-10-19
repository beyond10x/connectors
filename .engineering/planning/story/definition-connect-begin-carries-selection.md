---
format: aep.planning-md/3
id: story:definition-connect-begin-carries-selection
kind: story
status: draft
title: A definition connect's Begin carries the selection its question was asked for
summary: closes the window left after 439ed1a07 between the entry re-check and the owner Begin
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: apps/connectors/src/local/definition.rs
- confidence: inferred
  path: apps/connectors/src/local/session.rs
- confidence: inferred
  path: apps/connectors/tests/definition_connect_question.rs
- confidence: inferred
  path: apps/connectors/tests/failed_connect.rs
- confidence: inferred
  path: apps/connectors/tests/failed_connect_adversary.rs
- confidence: inferred
  path: apps/connectors/tests/owner_build_security.rs
- confidence: inferred
  path: contracts/cli/v1alpha1/owner.md
- confidence: inferred
  path: crates/connectors-build/src/ignored.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
revision: 4
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

## Scope

Derived 2026-10-19 by `story-scoper` on origin/main 0c6cbba7f. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** owner `Begin` protocol in `crates/connectors-host/src/local/owner` — cited (story names `transport.rs:415`)
- **Files:** `apps/connectors/src/local/definition.rs:1067-1077` (`connect()`: re-check, then `client.begin(alias, Some(profile), None, None)`) — cited
- **Files:** `crates/connectors-host/src/local/owner/transport.rs:415-450` (`Client::begin`, writes `Request::Begin`) — cited
- **Files:** `crates/connectors-host/src/local/owner/transport.rs:1771,1820-1870` (owner dispatch, `Request::Begin` handler) — inferred, the place to compare against `adapter.selection()` before the custody check at :1854
- **Files:** `crates/connectors-host/src/local/owner.rs:929-934` (`Request::Begin` fields; `deny_unknown_fields` at :38) — inferred
- **Symbols:** `Request::Begin`, `Client::begin`, `Adapter::selection()` (`crates/connectors-host/src/local/config.rs:373`), `Code::RevisionConflict` (`owner.rs:39`) — cited / inferred
- **Also likely:** `apps/connectors/src/local/session.rs:197,255`; `apps/connectors/tests/failed_connect.rs:182`, `failed_connect_adversary.rs:155`, `owner_build_security.rs:122` — inferred, other `Client::begin` callers if the signature changes
- **Also likely:** `apps/connectors/tests/definition_connect_question.rs` (race test at :399) and `crates/connectors-build/src/ignored.rs:342-348` — inferred
- **Documents:** `contracts/cli/v1alpha1/owner.md` (~:103), `CHANGELOG.md` — inferred
- **Confidence:** high for the two code sites; medium for test placement and caller fan-out
- **Would collide with:** any unit touching the owner `Request` enum, `Client::begin`, the `Begin` handler in `owner/transport.rs`, or `connect()` in `definition.rs`
- **Safety fact:** `Request` is `deny_unknown_fields` (`owner.rs:38`) and `Client::begin` calls `same_build()` (`transport.rs:421`) before writing `Begin`, so CLI and owner share the field set — unproven
