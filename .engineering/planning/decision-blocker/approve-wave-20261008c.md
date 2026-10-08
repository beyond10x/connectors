---
format: aep.planning-md/3
id: decision-blocker:approve-wave-20261008c
kind: decision-blocker
status: cleared
title: 'Approve wave 20261008c: Runpod pods bundle and invoke timeout stage'
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-08T16:02:35Z", actor: "human:timo", revision: 3}
---
## Question

Approve wave 20261008c: two units on integration branch `wave/20261008c`, one pull request, then release 0.36.0.

| unit | story | crates touched |
|---|---|---|
| U1 | `story:catalog-runpod-pods` | `connectors-catalog-provider` (adapters/catalog), `connectors-build`, `connectors-catalog` if the inventory refuses the duplicate `UpdatePod` |
| U2 | `story:invoke-timeout-after-dispatch-stage` | `connectors` (apps/connectors), `connectors-host`, ESS `cli` / `service_wire` |

## Options

| option | effect | cost |
|---|---|---|
| A | both units in parallel, one adversary pass on U1 (writes that bill money) | 2 trees, inside the 10G wave slot |
| B | U1 only; U2 next wave | the timeout fix waits a release |

Recommendation: A. The units share no file except `CHANGELOG.md`.

## Decision (2026-10-08)

Option A: both units in parallel, one adversary pass on U1, one integration branch and one pull request, release 0.36.0. No live call to the Runpod API: fixtures and the pinned OpenAPI document only; no account, key or spend.
