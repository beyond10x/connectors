---
format: aep.planning-md/3
id: story:cold-owner-start-verifies-once
kind: story
status: active
title: A cold owner start verifies the store once and answers describe within the caller's deadline
relations:
- informed_by: story:owner-start-outlasts-a-fixed-bound
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: .github/workflows/rust-gate.yml
- confidence: cited
  path: apps/connectors/src/local/operations.rs
- confidence: inferred
  path: apps/connectors/tests/owner_slow_start.rs
- confidence: cited
  path: contracts/cli/v1alpha1/owner.md
- confidence: cited
  path: crates/connectors-build/src/ignored.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/checkpoint_tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/metadata/er.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner.rs
- confidence: cited
  path: crates/connectors-host/src/local/owner/transport.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/store_cost_tests.rs
- confidence: inferred
  path: crates/connectors-host/src/local/runtime/state.rs
- confidence: cited
  path: docs/local-er-metadata.md
revision: 18
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T05:28:52Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-30T05:28:52Z", actor: "human:timo", revision: 4}
---
## Problem

0.59.3 lets a read wait for a slow owner start within the read's own deadline
(story:owner-start-outlasts-a-fixed-bound). The cold start is still slow enough to exceed a
consumer's own limit.

On 2026-10-30, a local consumer ran v0.59.3 against a 217 MB metadata store:
- A cold `operations describe` took 95.7 s and exited 0.
- The next read took 26.3 s, and the one after it 1.2 s.
- The consumer limits each call to 60 s.

The 0.59.3 change's author observed that every cold start checks the whole store twice: once
when the CLI admits the call itself, and again when the owner it starts verifies the store before
it answers its greeting. Only that observation backs this; no profile confirms it.

## Outcome

A cold owner start verifies the store once. `operations describe`, and every read that starts an
owner, answers within the caller's deadline on a store of that size.

## Acceptance

- One cold start, measured on a store of about 200 MB, verifies the store exactly once. A test
  counts the verification passes.
- On that store, a cold `operations describe` and a cold read each answer within 60 s on the CI
  runner class, and the next read takes no longer than a warm one.
- A store that fails verification still refuses before any provider call.

## Scope

Derived 2026-11-03 by `story-scoper` at 8150245c2. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** `crates/connectors-host/src/local/metadata` (fresh-open verification) — cited
- **Files:** `crates/connectors-host/src/local/metadata.rs:595` (`start_owner`), `:622` (`inspect`), `:987-1069` (`activate_er`) — cited
- **Files:** `crates/connectors-host/src/local/metadata/er.rs:2247-2287` (`open`: pool `take`, `er.start`, `resynchronize`), `:2787-2794` (`complete_snapshot`, the counted full replay), `:2718-2752` (`FULL_REPLAYS`, `simulate_process`, `exit_process`) — cited
- **Files:** `apps/connectors/src/local/operations.rs:110-130` (invoke: direct `admit_invoke`, then `connect_until`) — cited
- **Files:** `crates/connectors-host/src/local/owner/transport.rs:1535` (the owner's `Metadata::start_owner` before its greeting), `:316-331` (spawn, then wait for the greeting) — cited
- **Files:** `crates/connectors-host/src/local/owner.rs:505-614` (describe reads the store directly and starts an owner only on `DescriptionUnavailable`, `:541`), `:864` (`admit_invoke`) — cited
- **Test seam (counter):** `er::full_replays()` and `take_costs()["er.start"]` (re-exported at `metadata.rs:111,148`), with `simulate_process` standing for the CLI and owner processes, as `store_cost_tests.rs:570-641` and `checkpoint_tests.rs:174-184` use them; all `#[cfg(test)]` — cited
- **Test seam (binary):** a debug-build environment hook like `CONNECTORS_TEST_OWNER_START_DELAY_MS` (`transport.rs:1376-1380`, `:1452-1461`, `:1536-1543`) recording each fresh verification — inferred
- **Store generator:** `store_cost_tests.rs:144-178` (`grown_stores`) grows event count, not blob volume; a ~200 MB store needs a blob-heavy generator beside it — cited / inferred
- **Also likely:** `crates/connectors-host/src/local/runtime/state.rs:64-110`, `apps/connectors/tests/owner_slow_start.rs` — inferred
- **Gate/CI:** the store-cost tests are the `Timing` family in `crates/connectors-build/src/ignored.rs:140-147`; `.github/workflows/rust-gate.yml` runs the workspace in debug and no ignored `Timing` test — cited; the 60 s line needs a release-profile measurement — inferred
- **Documents:** `contracts/cli/v1alpha1/owner.md:127,176-177,248-256`, `docs/local-er-metadata.md:253-275` — cited
- **Confidence:** high for the defect site; medium for the fix surface (the `er.rs` open or the CLI's admission order)

### Diagnosis at scoping (code reading, not a trace)

A cold `operations invoke` opens and verifies the store in two processes: the CLI (`operations.rs:119` to `owner::admit_invoke`, `cached_direct` `owner.rs:607`, `Metadata::inspect`, `er::open` `er.rs:2247` with a full replay `er.rs:2305`) and the owner (`transport.rs:1535`). A fresh open of a store with no open checkpoint is itself two passes: the provider's own open, then the host's complete snapshot (`docs/local-er-metadata.md:253-263`, `er.rs:677-679`). A cold `operations describe` with a cached description starts no owner (`owner.rs:541`), so it verifies once; the reported 95.7 s describe fits the code only if its description was not cached — not established.
