---
format: aep.planning-md/3
id: decision-blocker:wave-20261010c-scope
kind: decision-blocker
status: cleared
title: Which parity units wave 20261010c delivers
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-10T04:32:44Z", actor: "human:timo", revision: 2}
---
## Question

Which parity units wave 20261010c delivers, by call count on `docs/fluxplane-plugin-parity.md`.

## Options

| option | what | cost |
|---|---|---|
| A | story:catalog-path-correction-and-value-bound (gitlab.search.blobs, 75 calls) and story:parity-gitlab-tags-releases (45) | one tree, catalog only |
| B | A plus story:parity-prometheus-query (69, a new native adapter) | one tree, units built in turn, at most 10G |
| C | story:parity-prometheus-query alone | no GitLab progress |

Recommended: B.

## Decided

Option B, 2026-10-10: story:catalog-path-correction-and-value-bound, then story:parity-gitlab-tags-releases (both change the GitLab selection), and story:parity-prometheus-query, in one tree built one unit at a time, at most 10G on disk. The engine change of the first unit gets an adversary review. The wave is released after it merges on green CI.
