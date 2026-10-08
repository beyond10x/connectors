---
format: aep.planning-md/3
id: story:runpod-create-start-command-and-volume
kind: story
status: active
title: Runpod pod.create admits the start command, entrypoint and network volume
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/providers/runpod/operations.json
- confidence: cited
  path: adapters/catalog/tests/runpod.rs
- confidence: cited
  path: adapters/catalog/tests/runpod_adversary.rs
- confidence: cited
  path: docs/catalog-runpod.md
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T18:14:40Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-08T18:14:40Z", actor: "human:timo", revision: 4}
---
## Outcome

`pod.create` in `adapters/catalog/providers/runpod/operations.json` admits the body keys `dockerStartCmd`, `dockerEntrypoint` and `networkVolumeId` of the pinned `PodCreateInput` schema, so a caller can start a pod with its own command and arguments and attach a network volume (a consumer starts every pod as one image with model arguments and keeps its weight cache on a network volume).

## Acceptance

- The selection's `body_keys` are the twelve reviewed keys plus these three; `adapters/catalog/tests/runpod.rs` pins the list.
- A create carrying all three is sent with them byte-identical in the request body.
- `docs/catalog-runpod.md` lists the three keys and no longer names `dockerStartCmd` or `networkVolumeId` as refused.
- A key outside the set (for example `templateId`) is still refused as `invalid_input` before any request.
