---
format: aep.planning-md/3
id: story:runpod-create-capacity-refusal
kind: story
status: implemented
title: A Runpod create refused for lack of capacity is a capacity refusal
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-17T03:03:00Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-17T03:03:00Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-17T03:03:01Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome
A Runpod `pod.create` that Runpod refuses for lack of capacity is classified `refused` with code
`capacity`, so a caller can start the same workload elsewhere instead of treating it as an
ordinary input refusal. Requested by a downstream consumer whose model profiles fall back to the
next profile on a capacity answer.

## Rule
Runpod documents no distinct stock-out answer: its REST document lists only `201` and `400`
for `createPod` (https://rest.runpod.io/v1/openapi.json, read 2026-10-17). Runpod's own clients
recognise one by text:
- runpodctl @e12397351fa1, `cmd/pods/createPods.go:124-135`: "no longer any instances available",
  on a 400 or 409;
- runpod-python `runpod/apps/api.py:148-157`: "resources to deploy your pod",
  "insufficient capacity", "no available machine", "out of stock".

A `pod.create` 4xx whose `message` (v1 error) or `detail` (v2, RFC 9457) contains one of these
texts, case-insensitively, is `refused`, code `capacity`. A 5xx stays `unknown` even with one of
these texts: the pod may exist. Every other answer keeps its classification.

## Acceptance
- Fixture answers built from those clients' texts (marked as such beside the fixtures, not as a
  captured answer): a 400 and a 409 with a v1 `message`, a 409 with a v2 `detail`, mixed case,
  each `refused` with code `capacity`; a 400 "Invalid input" stays `refused` without `capacity`;
  a 500 carrying a capacity text stays `unknown`.
- docs/catalog-runpod.md states the rule beside the `pod.create` rows and cites both sources.
- No live call and no spend. A real stock-out answer, when first seen, is recorded (status and
  body, redacted) as a fixture confirming the rule.
