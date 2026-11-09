---
format: aep.planning-md/3
id: story:metadata-conformance-emit-manifest-format
kind: story
status: active
title: metadata-conformance emit writes a manifest the pinned ess admits
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: inferred
  path: crates/connectors-build/src/metadata_conformance.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:09Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:10Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Outcome

`connectors-build metadata-conformance emit` writes a mutation manifest the pinned `ess` admits.

## Why

With ESS 0.57.0 (and, by reading the 0.56.0 source, likely before it), `emit` is refused:
`manifest.json: emit-swap/... is a emit-swap mutant, a class ess-mutation-manifest/2 does not
have; it is ess-mutation-manifest/4`. The emitter writes the `/2` format; the `emit-swap` class
exists only from `/4`. The gate runs only the baseline suite, which passes, and the unit test
`emitted_manifest_is_admitted_by_the_pinned_ess` passes, so nothing caught it. Not confirmed on a
0.56.0 build.

## Acceptance

- `emit` writes `ess-mutation-manifest/4` (suite digests, component, `out_of_scope`) or leaves out
  the classes `/2` cannot hold, and the pinned `ess` admits the result.
- A test runs `emit` end to end against the pinned `ess`, so a refusal fails it.
