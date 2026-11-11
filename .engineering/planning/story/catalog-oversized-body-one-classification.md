---
format: aep.planning-md/3
id: story:catalog-oversized-body-one-classification
kind: story
status: implemented
title: An over-bound catalog response body has one classification in contract, model and transport
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: contracts/catalog/v1alpha1/semantics.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:59:46Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:59:47Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "active", to: "implemented", at: "2026-11-11T06:18:55Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Problem

`contracts/catalog/v1alpha1/semantics.md` §5–6 classifies a provider response body over the response bound as `UpstreamProtocol`, while the catalog transport and the shared model refuse it as `capacity` (the GitLab `file.raw` read shipped in 0.53.0 follows the transport: a file over the 4 MiB bound is refused `capacity`). The contract and the implementation disagree.

## Acceptance

- The contract, the shared ESS model and the transport name one classification for an over-bound response body, and a conformance scenario holds it for a catalog read.
