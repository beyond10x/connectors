---
format: aep.planning-md/1
id: story:claude-oauth-deployment-compatibility
kind: story
status: active
title: Preserve the deployed Connector contract while repairing OAuth
summary: Backport the validated OAuth failure classification without introducing newer unrelated protocol fields.
relations:
- derived_from: epic:subscription-custody
scope:
- confidence: cited
  path: crates/server/src/hosted.rs
- confidence: cited
  path: crates/server/src/hosted/subscription.rs
- confidence: cited
  path: crates/subscription-custody/src/lib.rs
revision: 4
---
## Outcome

Existing consumers can adopt the OAuth recovery fix without migrating their generated service SDK protocol.

## Evidence

The latest Connector protocol adds an OperationDescription rate_advice field not implemented by the consuming product's pinned generated service SDK. The local composed build refuses this mismatch before deployment. This compatibility revision starts from the product's existing pinned Connector source and applies only the reviewed custody and hosted error correction from pull request 20.

## Acceptance

Custody and hosted regression tests pass; no protocol, runtime configuration, dependency, or catalog change enters the patch. The consuming product compiles and passes its composed local acceptance before promotion.

## Scope

Existing subscription-custody and hosted subscription HTTP error projection plus their regression tests.
