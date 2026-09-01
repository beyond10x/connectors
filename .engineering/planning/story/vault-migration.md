---
format: aep.planning-md/1
id: story:vault-migration
kind: story
status: implemented
title: Provide a safe Vault migration command
summary: Migrate existing custody without automatic source deletion.
relations:
- derived_from: epic:remote-secrets
revision: 4
---
## Acceptance

A bounded, idempotent operator command copies scoped Vault values into Secrets entirely in memory, reports references only, and never removes the source.
