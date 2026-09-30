---
format: aep.planning-md/3
id: story:catalog-post-reads
kind: story
status: draft
title: A selection may declare a documented POST query as a read
relations:
- decomposes: epic:hubspot-crm-reads
revision: 1
---
## Outcome

A selection may declare a POST operation `effect: read` when the provider documents it as a query
that changes nothing (HubSpot `POST /crm/objects/2026-09/{objectType}/search` and `/batch/read`),
so it runs without an approval. Today the engine admits a read only for GET
(`adapters/catalog/src/lib.rs:239-247`).

## Why

HubSpot's list read has no time filter; `search` is the only CRM Objects operation that filters
by `hs_lastmodifieddate`, so deltas need it (`epic:hubspot-crm-reads`).

## Open

How the selection proves the POST is a read: a reviewed per-selection flag, and what the approval
model says about a body the caller supplies to a read. Needs a design decision before scheduling.
