---
format: aep.planning-md/3
id: story:runpod-billing-pods-read
kind: story
status: implemented
title: The Runpod catalog reads pod billing (billing.pods)
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T02:29:04Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-17T02:29:05Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-17T02:29:06Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome
The Runpod catalog provider offers the read `billing.pods` (`PodBilling`, `GET /billing/pods`,
pinned document `adapters/runpod/upstream/runpod-rest-v1.json`), so a caller can read the
account's reported pod spend before starting a pod. Today the selection
(`adapters/catalog/providers/runpod/operations.json`) offers only pod.create, pods.list and
pod.terminate.

## Acceptance
- `billing.pods` is a declared `read` of the Runpod selection with the query parameters the pinned
  document declares (bucketSize, endTime, gpuTypeId, grouping, podId, startTime) and the same
  credential and approval posture as `pods.list` (no approval for a read).
- A fixture test, red before the selection entry exists, invokes `billing.pods` against a local
  fixture and returns the fixture's billing records unchanged; an undeclared query parameter is
  refused as `invalid_input` before any request. No live Runpod call.
- The generated references (`connectors-docs generate`) and the Runpod guide name the operation.
