---
format: aep.planning-md/1
id: specification:catalog-write-discovery-already-present
kind: specification
status: draft
title: Catalog write discovery is already present in v0.7.0
relations:
- specifies: story:catalog-write-discovery
revision: 1
---
# Catalog write discovery is already present

Read-only review of released Connectors source `4d0cd30872533da40f209274f936eaeae9bf01d7` on 2026-09-07 finds the requested discovery behavior already implemented. Search and describe consider admitting credential bindings; invocation still enforces the explicitly selected credential.

The evidence is `crates/integration-catalog/src/lib.rs:379,416,482`, regression cases in `crates/integration-catalog/src/tests.rs:406,429,447`, and local CLI coverage in `crates/connectors-runtime/tests/local_catalog_writes.rs:240`. The related story already records its delivered acceptance and historical gate. No new tests or live operations ran during this review.

Do not schedule another implementation of `story:catalog-write-discovery` in the small wave. Its draft lifecycle status is stale; this finding records the discrepancy without fabricating fresh execution evidence or silently walking lifecycle rungs.
