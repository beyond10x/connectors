---
format: aep.planning-md/3
id: story:served-admission-admits-a-set-of-service-accounts
kind: story
status: active
title: A served host admits a configured set of service accounts, or a namespace and a name pattern
summary: 'D275: one instance serves several agent service accounts.'
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: contracts/service/compatibility.md
- confidence: cited
  path: crates/connectors-host/src/server.rs
- confidence: cited
  path: crates/connectors-host/src/server/token_review.rs
- confidence: cited
  path: crates/connectors-host/tests/http_service_account_token.rs
- confidence: cited
  path: crates/connectors-host/tests/http_service_account_token_review.rs
- confidence: cited
  path: docs/running-services.md
- confidence: inferred
  path: ess/domains/declarations.yaml
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T13:43:02Z", actor: "human:timo", revision: 11, executor: "agent:conn-d275-w621", correlation: "wave-d275-0621"}
- {from: "proposed", to: "active", at: "2026-10-30T13:43:03Z", actor: "human:timo", revision: 12, executor: "agent:conn-d275-w621", correlation: "wave-d275-0621"}
---
# Story: served-admission-admits-a-set-of-service-accounts

## Outcome

An operator who runs several agent pods under different service accounts admits all of them to one
served instance, by a list of accounts or by a namespace and a name pattern, instead of running one
instance per account.

## Context

D275, from the live-cluster rehearsal of 0.62.0. The `service-account-token` admission
(story:served-instance-admits-a-service-account-token) admits exactly one `principal`
(`crates/connectors-host/src/server/token_review.rs`, `decide` and `valid_principal`). Documented in
`docs/running-services.md` (the TokenReview example near line 36).

## Acceptance

- `principal` (one account) is admitted exactly as before.
- `principals`, a non-empty list of `system:serviceaccount:<namespace>:<name>`, admits each listed
  account and no other.
- `service_accounts: {namespace, name}` admits an account in that namespace whose name matches
  `name`, a pattern whose only wildcard is `*`; the namespace is literal.
- Exactly one of the three is configured; none, two, an empty list, an invalid entry or an empty
  pattern refuse at start with a message naming the key.
- The reviewed username is what the audit anchor records as the principal.
- docs/running-services.md describes the three forms.

## Out of Scope

- Groups, or admission by anything other than the reviewed username.
- A wildcard in the namespace.

## Ambiguities

None.

## Open Questions

None.

## Scope

Derived 2026-10-30 by `story-scoper`, condensed by the coordinator. Every line is **cited** or **inferred**.

- **Files:** `crates/connectors-host/src/server/token_review.rs:71-125` (`ServiceAccountAdmission`), `:138-180` (`admit`), `:189-215` (`decide`), `:256-283` (`valid_principal`) — cited
- **Files:** `crates/connectors-host/src/server.rs:60-185` (`ServiceConfigV3` schema, `principal` field, `admission()`), `:251-286` (`Admission::admit`, the principal the anchor records at `:282`) — cited
- **Tests:** `crates/connectors-host/tests/http_service_account_token.rs`, `http_service_account_token_review.rs` (`:812-814` refuses a list under `principal`, kept) — cited
- **Documents:** `docs/running-services.md:22-55` (Admission) — cited; `contracts/service/compatibility.md:50-51,58,88-89` and its mirror `website/docs/reference/contracts/compatibility.md`, `ess/domains/declarations.yaml:120-175` — inferred
- **Confidence:** high
- **Would collide with:** the review-token unit in `token_review.rs` (it owns only the credential read at `:98-132`), the state-message unit in `server.rs` (it owns only `:459-469`)
- **Safety fact:** the anchor and the git relay take the principal from `Admission::admit` (`server.rs:282`, `adapters/catalog/src/served/git.rs:442-446`), so returning the reviewed username there needs no catalog change — unproven
