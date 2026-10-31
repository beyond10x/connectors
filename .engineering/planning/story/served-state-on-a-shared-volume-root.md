---
format: aep.planning-md/3
id: story:served-state-on-a-shared-volume-root
kind: story
status: implemented
title: A host whose state sits on a world-writable volume root says what to change
summary: 'D275: state on a 0777 PVC root is refused without saying which directory or why.'
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-host/src/local/filesystem.rs
- confidence: inferred
  path: crates/connectors-host/src/local/mod.rs
- confidence: cited
  path: crates/connectors-host/src/local/service.rs
- confidence: cited
  path: crates/connectors-host/src/server.rs
- confidence: inferred
  path: docs/local-container.md
- confidence: cited
  path: docs/running-services.md
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T13:43:05Z", actor: "human:timo", revision: 11, executor: "agent:conn-d275-w621", correlation: "wave-d275-0621"}
- {from: "proposed", to: "active", at: "2026-10-30T13:43:06Z", actor: "human:timo", revision: 12, executor: "agent:conn-d275-w621", correlation: "wave-d275-0621"}
- {from: "active", to: "implemented", at: "2026-10-30T15:04:46Z", actor: "human:timo", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: served-state-on-a-shared-volume-root

## Outcome

An operator whose persistent volume root is world-writable (`0777`) learns from the host's refusal
which directory is refused and what to set instead, and the docs show a layout that starts.

## Context

D275, from the live-cluster rehearsal of 0.62.0. `admitted_directory`
(`crates/connectors-host/src/local/filesystem.rs:60-70`) refuses a group- or other-writable
directory as `InvalidConfiguration`, so `state` on a `0777` PVC root does not start, and the message
does not say which directory or why.

## Acceptance

- The start refusal for a refused state directory names that directory (the operator's own
  configured path or one of its ancestors) and the reason: not owned by the host user or root, or
  group- or other-writable, or (for `state` itself) not private. No OS error text is carried.
- No admission is loosened: a world-writable `state` or ancestor is still refused.
- docs/running-services.md says that every ancestor of `state` is checked, and how to run on a
  volume whose root is `0777`: mount a subdirectory (`subPath`) or have an init container set the
  root to `0755` or stricter, owned by the host user or root.
- A test starts a host whose `state` sits under a `0777` directory and checks the message names it.

## Out of Scope

- Accepting a world-writable state directory.

## Ambiguities

None.

## Open Questions

None.

## Scope

Derived 2026-10-30 by `story-scoper`, condensed by the coordinator. Every line is **cited** or **inferred**.

- **Files:** `crates/connectors-host/src/local/filesystem.rs:60-71` (`admitted_directory`), `:95-127` (`directory`: `/` at `:98`, every ancestor at `:122` with `private=false`, the final directory at `:125`, `mkdirat 0700` at `:108`) — cited
- **Files:** `crates/connectors-host/src/server.rs:459-469`, where every `ServiceState::open` failure becomes `service state cannot be opened for this instance` — cited
- **Files:** `crates/connectors-host/src/local/service.rs:278`; `crates/connectors-host/src/local/mod.rs:46-60` (`Failure`, carries no OS text) — cited / inferred
- **Documents:** `docs/running-services.md` (state examples `:39,105,179`), `docs/local-container.md:37-39` — cited / inferred
- **Confidence:** high
- **Safety fact:** every ancestor is checked, so a `0700` `state` under a `0777` volume root is refused at the root; the documented layout must keep the `0777` root out of the path (a `subPath` mount, or an init container that sets the root to `0755` or stricter) — unproven
