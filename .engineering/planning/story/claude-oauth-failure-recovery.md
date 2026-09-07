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
revision: 4
---
## Outcome

An OAuth token endpoint rate limit never becomes an invalid authorization code. A failed exchange reports a bounded credential-free cause and does not silently invite resubmission of consumed flow state.

## Evidence

The shipped custody exchange collapses every non-success status into OauthRefused. A bounded invalid-code diagnostic received provider HTTP 429, while the product displayed an expired-code message. Pending state is consumed before the network exchange, so retrying that same flow cannot recover.

## Acceptance

Exercise successful PKCE and refresh, provider HTTP 429, provider 5xx, invalid and replayed codes through existing custody and hosted service tests. Preserve tenant and subject binding, single-use state, secret custody, and no provider response-body exposure. Map rate limits to HTTP 429 for existing SDK consumers.

## Scope

Existing subscription custody exchange and hosted subscription error projection. Devcenter owns the recovery UI and composed deployment proof in its related model credential story.

## Verified implementation and release reconciliation

Runtime and test source is unchanged from 1c45d5bfbdc583738e90f2b93cf9c124ed2c5568, based on the exact 0.7.0 release. Its complete source and native matrix passed in run 34100114935. That source revision is retained on the published compatibility branch for immutable downstream consumption. Recreate this owning record through AEP on current main to incorporate the subsequent verified release-delivery record without editing journal text. The prior original implementation also passed run 34096048843. Current branch CI remains required before merge.

This single existing story is already authorized for active implementation. Reconciliation restores its draft, proposed and active lifecycle through the CLI. Successful real user authorization and model replies remain Devcenter acceptance work; fixture-level OAuth regressions do not claim that external proof.
