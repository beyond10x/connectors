---
format: aep.planning-md/1
id: story:claude-oauth-failure-recovery
kind: story
status: active
title: Recover truthfully from Claude OAuth failures
summary: Distinguish provider rate limits from invalid authorization and make consumed flows recoverable.
relations:
- derived_from: epic:subscription-custody
scope:
- confidence: cited
  path: crates/server/src/hosted.rs
- confidence: cited
  path: crates/server/src/hosted/subscription.rs
- confidence: cited
  path: crates/subscription-custody
revision: 5
---
## Outcome

An OAuth token endpoint rate limit never becomes an invalid authorization code. A failed exchange reports a bounded credential-free cause and does not silently invite resubmission of consumed flow state.

## Evidence

The shipped custody exchange collapses every non-success status into OauthRefused. A bounded invalid-code diagnostic received provider HTTP 429, while the product displayed an expired-code message. The pending state is consumed before the network exchange, so retrying that same flow cannot recover.

## Acceptance

Exercise successful PKCE and refresh, provider HTTP 429, provider 5xx, invalid and replayed codes through existing custody and hosted service tests. Preserve tenant and subject binding, single-use state, secret custody, and no provider response-body exposure. Map rate limits to HTTP 429 for existing SDK consumers.

## Scope

Existing subscription custody exchange and hosted subscription error projection. Devcenter owns the recovery UI and composed deployment proof in its related model credential story.


## Mainline reconciliation

The original published candidate d4728cf5 passed the complete source and native matrix in run 34096048843. Main subsequently advanced to prepare version 0.7.0. Reapply the unchanged three-file OAuth patch onto that main and recreate this owning record through the AEP CLI, retaining the original published commit and its validation evidence. No runtime implementation is changed by this reconciliation. The deployed Devcenter compatibility pin stays separate and unchanged. New mainline CI must pass before merge.

## Reconciled-source verification

The three runtime/test source files are byte-identical to the original fully tested candidate d4728cf5. Targeted subscription-custody and server tests plus all-target clippy and formatting passed against current main e80b7ae1. The AEP store was recreated through CLI operations on that base rather than resolving journal text manually. The existing PR will be updated with an exact expected-head lease; its old published commit is retained by a local backup ref. Full branch CI remains required before merge.
