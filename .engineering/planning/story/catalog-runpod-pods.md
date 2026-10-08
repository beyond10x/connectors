---
format: aep.planning-md/3
id: story:catalog-runpod-pods
kind: story
status: active
title: Runpod pods created, listed and terminated through the catalog provider
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-08T16:02:35Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-08T16:02:35Z", actor: "human:timo", revision: 3}
---
## Outcome

A Runpod connection through the catalog provider creates, lists and terminates pods, with the
Runpod API key held in the Connectors keyring. A downstream gateway that starts GPU pods on demand
invokes these three operations instead of carrying its own HTTP client for the Runpod REST API and
holding the account key itself.

## Source

Runpod's REST API OpenAPI document, <https://rest.runpod.io/v1/openapi.json>, retrieved
2026-10-08: OpenAPI `3.0.3`, `info.version` `0.1.0`, 154,609 bytes, SHA-256
`9500a8989878d53d8731f27bf8dbbd57801b328c760bdb32c38ba36d5cb580db`, server
`https://rest.runpod.io/v1`, one security scheme `ApiKey` (`http`, `bearer`). Pin it under
`adapters/runpod/upstream/` with a source-hash record, as the Zendesk and GitLab sources are pinned;
run the redaction check and redact only if gitleaks or the hooks find example credentials.

Known shape of the pinned document, to handle and not to work around:

- `UpdatePod` is the `operationId` of both `PATCH /pods/{podId}` and `POST /pods/{podId}/update`.
  Neither is selected, but the inventory must not refuse the document for the duplicate; if it
  does, record the gap and decide in the inventory, not by editing the pinned source.
- There is no user or account endpoint. The connection's identity and readiness probe is
  `GET /pods` (`ListPods`) unless the implementor finds a better read; the subject is then the
  configured connection, not an account id, and the docs say so.

## Operations

| id | `operationId` | endpoint | effect |
|---|---|---|---|
| `pod.create` | `CreatePod` | `POST /pods` | write |
| `pods.list` | `ListPods` | `GET /pods` | read |
| `pod.terminate` | `DeletePod` | `DELETE /pods/{podId}` | write |

`pod.show` (`GetPod`, `GET /pods/{podId}`) may be added as a read if the terminate guard or the
consumer's orphan sweep needs it; nothing else is selected.

## Write semantics

- Both writes are required-approval mutations under the existing approval policy; the docs show
  the policy entry that admits them for an unattended caller.
- `pod.create`: a definite refusal (a documented 4xx answer) is `refused`; a timeout, a 5xx or a
  lost connection after the request was sent is `outcome_unknown`, never `refused`. This is the
  double-billing guard the consumer depends on: an unknown create must not be retried blindly.
- `pod.terminate`: a 404 on an already-gone pod is reported as the provider's answer; the docs
  state what the caller should conclude from it.
- `body_keys` closes the create body to the keys the consumer needs (at least `name`,
  `imageName`, `gpuTypeIds`, `gpuCount`, `containerDiskInGb`, `ports`, `env`, `cloudType`); the
  implementor reads the pinned `PodCreateInput` schema and lists the final set in the docs.

## Shared surfaces

- Own files: `adapters/catalog/providers/runpod/operations.json`,
  `adapters/catalog/generated/bundles/runpod.bundle.json`, `adapters/runpod/upstream/*`,
  `adapters/catalog/tests/runpod.rs`, `docs/catalog-runpod.md`.
- Shared: `adapters/catalog/generated/bundles/index.json`, `adapters/catalog/tests/bundle_drift.rs`
  (one row each), `crates/connectors-build/src/gate.rs` and `source_hashes.rs` if the pinned-source
  list is enumerated there, `CHANGELOG.md`, `README.md` support table.
- The auth profile is the existing bearer profile; no new auth code is expected.

## Acceptance

- `operations.json` exposes exactly `pod.create`, `pods.list` and `pod.terminate` (plus `pod.show`
  if added), with the effects above; `adapters/catalog/tests/runpod.rs` pins the id list with each
  `operationId`, method and path.
- The committed bundle regenerates without drift from the pinned source; the gate checks the
  pinned digest.
- A test against a local fake Runpod server: create answered 200 returns the pod; create answered
  400 is `refused`; create whose server drops the connection after reading the request is
  `outcome_unknown`; terminate answered 200 and 404 each report as documented; list returns the
  pods.
- A create body carrying a key outside `body_keys` is refused as `invalid_input` before any request.
- `docs/catalog-runpod.md` gives setup (API key into the keyring), the approval policy entry for the
  two writes, and one example invocation per operation.
- Spec first: any ESS change the provider needs (provider id, profile, approval entries) is declared
  in the repository's ESS specification and validated with the pinned `ess` before code.
