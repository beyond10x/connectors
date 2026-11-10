---
format: aep.planning-md/3
id: story:live-acceptance-without-serve
kind: story
status: active
title: The live acceptance runner runs without the removed serve command
relations:
- informed_by: story:connectors-do-short-invocation
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: crates/connectors-conformance/src/main.rs
- confidence: inferred
  path: docs/live-e2e.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:59:41Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:59:42Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Problem

0.60.0 removes `connectors serve` (story:connectors-do-short-invocation, D9), but the live
acceptance runner in `crates/connectors-conformance` still describes a federated gateway at
`127.0.0.1:17100` before any scenario group runs. Nothing serves that gateway any more, and the
runner has no option to skip the federated groups, so a live run cannot complete
(`docs/live-e2e.md`).

## Outcome

The live acceptance runner runs again. It either hosts the gateway itself through the
`connectors_host::federation` library, which is still shipped, or it runs the federated groups only
when a gateway address is given and reports them as not run otherwise.

## Acceptance (draft)

- A live run with no gateway runs every non-federated group and reports the federated ones as not
  run, by name.
- A live run with a library-hosted gateway runs the federated groups.
