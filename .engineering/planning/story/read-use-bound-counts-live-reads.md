---
format: aep.planning-md/3
id: story:read-use-bound-counts-live-reads
kind: story
status: active
title: The concurrent read-use bound counts only unreleased uses
tags:
- cr-004
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: crates/connectors-host/src/local/registry/reuse.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/revalidation.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/use_and_retirement.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T09:12:04Z", actor: "human:operator", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-10T09:12:05Z", actor: "human:operator", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Defect

Code review finding CR-004 (S2): The "1,000 concurrent read uses" bound counts finished reads.

- Citations: `crates/connectors-host/src/local/registry/use_and_retirement.rs:231-235` (`SELECT count(*) FROM registry_uses` with no `released` filter), `:273` (release sets a flag, deletes nothing); same pattern at `registry/reuse.rs:170-187`, `registry/revalidation.rs:371-373`; `docs/local-connection-registry.md:221`.
- What happens: each read leaves its row until its deadline (30 s default, `apps/connectors/src/local/invoke.rs:52`). Over 1000 completed reads in that window make later reads, reuse captures and revalidations fail `capacity`.
- Status: logic verified; reachability of about 33 reads per second is inferred.
- Fix: count `released=0` only, or delete on release.

- Review's story note: new.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-004. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- More than 1,000 released read uses inside one deadline window do not refuse the next read, reuse capture or revalidation `capacity`; 1,000 unreleased uses still do; a registry test is red on the base.
- If no test can make it red, that evidence is recorded and nothing is changed.
