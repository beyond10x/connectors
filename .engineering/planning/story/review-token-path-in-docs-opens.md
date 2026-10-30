---
format: aep.planning-md/3
id: story:review-token-path-in-docs-opens
kind: story
status: implemented
title: The documented review credential_file path is one the host opens
summary: 'D275: the documented serviceaccount/token path is a symbolic link the host refuses.'
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
- {from: "draft", to: "proposed", at: "2026-10-30T13:43:05Z", actor: "human:timo", revision: 11, executor: "agent:conn-d275-w621", correlation: "wave-d275-0621"}
- {from: "proposed", to: "active", at: "2026-10-30T13:43:05Z", actor: "human:timo", revision: 12, executor: "agent:conn-d275-w621", correlation: "wave-d275-0621"}
- {from: "active", to: "implemented", at: "2026-10-30T15:16:18Z", actor: "human:timo", revision: 13, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: review-token-path-in-docs-opens

## Outcome

An operator who copies the TokenReview example from the docs gets a host that opens its review
token.

## Context

D275, from the live-cluster rehearsal of 0.62.0 (inferred, not run). `docs/running-services.md:46`
gives `credential_file: /var/run/secrets/kubernetes.io/serviceaccount/token`. In a projected volume
that name is a symbolic link to `..data/token`, and `read_secret_file`
(`crates/connectors-host/src/credentials.rs:100-107`) opens with `O_NOFOLLOW`, which refuses a
symbolic link as the last component.

## Acceptance

- The code keeps refusing a symbolic link as the last path component; intermediate links are
  followed as today (coordinator decision: no projected-layout walk, which would refuse an image
  whose `/var/run` links to `/run`).
- A test builds the projected layout (`..<timestamp>/token`, `..data -> ..<timestamp>`,
  `token -> ..data/token`): `.../token` is refused at start with the current message and
  `.../..data/token` is read; after `..data` is swapped to a new timestamp directory the next
  review reads the new token.
- The documented example names `/var/run/secrets/kubernetes.io/serviceaccount/..data/token` and
  says in one sentence why the conventional name is refused.

## Out of Scope

- Following arbitrary symbolic links.

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
