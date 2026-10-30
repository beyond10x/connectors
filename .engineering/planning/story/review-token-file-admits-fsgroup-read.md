---
format: aep.planning-md/3
id: story:review-token-file-admits-fsgroup-read
kind: story
status: implemented
title: The review token file is admitted at 0640 when its group is one the process holds
summary: 'D275: a pod with fsGroup sees the projected token as 0640 and the host refuses it.'
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: contracts/service/compatibility.md
- confidence: cited
  path: crates/connectors-host/src/credentials.rs
- confidence: cited
  path: crates/connectors-host/src/server/token_review.rs
- confidence: inferred
  path: crates/connectors-host/tests/http_service_account_token_review.rs
- confidence: inferred
  path: crates/connectors-host/tests/service.rs
- confidence: cited
  path: docs/running-services.md
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T13:43:04Z", actor: "human:timo", revision: 11, executor: "agent:conn-d275-w621", correlation: "wave-d275-0621"}
- {from: "proposed", to: "active", at: "2026-10-30T13:43:04Z", actor: "human:timo", revision: 12, executor: "agent:conn-d275-w621", correlation: "wave-d275-0621"}
- {from: "active", to: "implemented", at: "2026-10-30T15:16:16Z", actor: "human:timo", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: review-token-file-admits-fsgroup-read

## Outcome

A served host that runs in a pod with `fsGroup` reads its kubelet-projected review token, instead of
refusing it as not private.

## Context

D275, from the live-cluster rehearsal of 0.62.0. `read_secret_file`
(`crates/connectors-host/src/credentials.rs:100-125`) requires mode bits `& 0o077 == 0` and the
effective user as owner. With `fsGroup` the kubelet presents the projected token as `0640` with the
pod's fsGroup as group. `docs/local-mounted-secret-custody.md:86-89` already admits group read for a
mounted secret on the same terms, and recommends `fsGroup`.

## Acceptance

- `0600` owned by the effective user is admitted as before.
- The review token (`token_review.credential_file`) is also admitted at `0640` when its group is
  the effective group or a supplementary group of the process, it is not group-writable and has no
  other bits, and its owner is the effective user or root.
- `0660`, `0644`, `0604`, and a `0640` whose group the process does not hold are refused, with the
  current message.
- Every other credential file read through `read_secret_file` (static bearer, federation, upstream
  `credential`, SQL `password`, conformance `--token-file`) keeps the `0600` rule; a test shows a
  `0640` static-bearer credential is still refused.
- The rule is written where the review token is documented (docs/running-services.md).

## Out of Scope

- Changing the mounted-secret custody rules.
- Windows.

## Ambiguities

None.

## Open Questions

None.

## Scope

Derived 2026-10-30 by `story-scoper`, condensed by the coordinator. Every line is **cited** or **inferred**. One unit with story:review-token-path-in-docs-opens.

- **Files:** `crates/connectors-host/src/credentials.rs:100-125` (`read_secret_file`, predicate `:109-118`) — cited
- **Files:** `crates/connectors-host/src/server/token_review.rs:64-68,98-132` (`TokenReviewConfig::credential_file`, its read) — cited
- **Reusable:** `crates/connectors-custody/src/mounted.rs:118-162` (`Identity::current`, `Identity::holds`, `pub(crate)`) — cited
- **Tests:** `crates/connectors-host/tests/service.rs:217-244`, `http_service_account_token_review.rs:730-776`, `http_service_account_token.rs:645-658` — cited
- **Documents:** `docs/running-services.md:11,46,57`; `contracts/service/compatibility.md:51` — cited
- **Confidence:** high
- **Safety fact:** `read_secret_file` also reads the static-bearer `service_credential`, federation credentials, `HttpConfig.credential`, the SQL `password` and the conformance `--token-file` (`credentials.rs:57,147`); the coordinator's decision keeps the group-read rule to the review token only — cited
