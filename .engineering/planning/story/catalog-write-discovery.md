---
format: aep.planning-md/1
id: story:catalog-write-discovery
kind: story
status: draft
title: Catalog writes remain discoverable across connection order and declare approval
relations:
- derived_from: epic:post-m1
revision: 2
---
## Objective

O1: governed reach. A read-only first connection must not hide a write admitted by a later connection. Mutating and destructive operations must declare required approval consistently in search and describe.

## Acceptance

Mixed read-only and writable bindings expose each operation once with all admitting identities. Read-only-only deployments still refuse writes; invocation continues to validate the explicitly chosen connection. Regression tests cover ordering and every admitted operation's effect/approval pair. No approval evaluation moves into the catalog adapter.

## Implementation

The catalog adapter now checks any admitting connection at description time and projects approval posture from declared effects. Named credential test fixtures use neutral bot labels. The focused integration-catalog suite passes all 35 tests; the full repository gate is required before publication.

## Validation

All twelve Cargo workspaces pass their locked tests, including the runtime's no-default-features lane. The catalog lock, links, story index and ESS projection checks pass. The final ESS check used the current source-built CLI because the older installed binary predates the specify command. Focused clippy passes with warnings denied. Live discovery correctly returns a mutating operation with required approval and only its writable connection.
