---
format: aep.planning-md/3
id: story:catalog-name-check-reads-nested-sources
kind: story
status: implemented
title: The provider-name check reads every catalog source file, nested ones too
summary: no_rust_names_a_declared_provider fails on PR 224 because adapters/catalog/src now holds a served/ directory; it must walk into it and check served/git.rs too.
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: crates/connectors-build/src/catalog_feed_conformance.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T12:49:15Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}, executor: "agent:conn-d265-c4", correlation: "wave-d265-c4"}
- {from: "proposed", to: "active", at: "2026-10-30T12:49:16Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}, executor: "agent:conn-d265-c4", correlation: "wave-d265-c4"}
- {from: "active", to: "implemented", at: "2026-10-30T12:49:16Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}, executor: "agent:conn-d265-c4", correlation: "wave-d265-c4"}
---
# Story: catalog-name-check-reads-nested-sources

## Outcome

A maintainer opening a pull request that adds a module directory under `adapters/catalog/src` gets a
provider-name check that reads the new files, instead of a gate that fails on reading a directory.

## Context

PR 224 (wave/d265-c4) added `adapters/catalog/src/served/git.rs`. The test
`catalog_feed_conformance::no_rust_names_a_declared_provider`
(`crates/connectors-build/src/catalog_feed_conformance.rs:1197`) lists `adapters/catalog/src` with
one `read_dir` and reads every entry as a file, so CI run 41047189935 failed with `IsADirectory`
at line 1215. The local gate of the wave did not run connectors-build's tests.

## Acceptance

- The check reads every `.rs` file under `adapters/catalog/src` at any depth, `served/git.rs` among
  them, and still reads `catalog_feed_conformance.rs` itself.
- It fails when a nested file names a declared provider, a profile or an operation it reads, in any
  letter case; a test proves the walk reaches a nested file.
- `cargo test -p connectors-build --bin connectors-build catalog_feed_conformance` passes.

## Out of Scope

- Widening the check to other crates.
- Changing which names count as declared.

## Ambiguities

None.

## Open Questions

None.
