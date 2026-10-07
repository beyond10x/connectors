---
format: aep.planning-md/3
id: story:feed-profile-capabilities
kind: story
status: draft
title: A feed profile declares what its provider observes, and the suite tests that
relations:
- decomposes: epic:generic-datasource-feeds
- serves: vision:independent-contract-adapters
- depends_on: story:feed-contract
scope:
- confidence: inferred
  path: contracts/datasources/feed/v1alpha1
- confidence: inferred
  path: crates/connectors-build/src/feed_conformance.rs
- confidence: inferred
  path: ess/domains/feed.yaml
revision: 2
---
## Outcome

A feed profile declares what its provider can observe, and the family's conformance suite tests a binding against what its profile declares rather than against the strongest provider: whether deletions are observed, where a container's `kind` comes from, and how a revision is formed.

## Why

Wave 20261006d (2026-10-06): the GitLab binding passed 21 of 28 scenarios with no defect in it. The suite always expects a tombstone, while `contracts/datasources/feed/v1alpha1/semantics.md:134-135` lets a profile observe no deletions (GitLab deletes merge requests outright); it expects the stored `kind`, while GitLab's word is always `project`; and its restore scenario reuses an `updated_at`, which is GitLab's revision.

## Work

- Contract and model (`ess/domains/feed.yaml`): a profile's capabilities — `deletions: observed | not-observed`, `kind: provider-word | fixed-word`, `revision: opaque | update-time` — carried in each binding's descriptor.
- The suite synthesizes or skips the deletion, kind and restore scenarios from those capabilities, and a binding cannot claim a capability its declaration does not support.

## Acceptance

- A fixture binding declaring `deletions: not-observed` passes the suite with the tombstone scenarios skipped and named as skipped in the report; declaring `observed` and dropping tombstones fails it.
- The native fixture and the two catalog fixture providers still pass every scenario they declare.
