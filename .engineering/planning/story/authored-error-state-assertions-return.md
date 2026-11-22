---
format: aep.planning-md/3
id: story:authored-error-state-assertions-return
kind: story
status: draft
title: Authored scenarios assert the state an error reports again, once the model can fill it
summary: 43 error state assertions were dropped because no ess/19 source fills an error's state
tags:
- from-finding
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
- informed_by: story:authored-scenarios-known-failing
revision: 1
---
## Context

story:authored-scenarios-known-failing (merge 32b7edf45) made all 52 authored ESS scenarios pass. To
do that it dropped 43 `state` assertions from error blocks in the credential_evidence,
idempotency, mutations, refresh and sessions scenarios.

The model declares an error's `state` field, but no source fills it. The only source that could,
`{subject: state}`, needs ess/23. The Entity Runtime lowering that generates
`entity-runtime-definitions.json` refuses ess/23. Each scenario's view assertion still checks the
final state, but nothing checks the state an error reports.

## Acceptance

- Once the pinned ess and the Entity Runtime lowering both admit a source for an error's `state`,
  the model fills it, and the 43 dropped assertions are restored. Every authored scenario still
  passes.
- Until then, this story stays a draft behind that upstream support.
