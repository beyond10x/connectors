---
format: aep.planning-md/3
id: story:runpod-billing-pods-read
kind: story
status: draft
title: The Runpod catalog reads pod billing (billing.pods)
revision: 1
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
