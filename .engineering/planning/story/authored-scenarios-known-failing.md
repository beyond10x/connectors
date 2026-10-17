---
format: aep.planning-md/3
id: story:authored-scenarios-known-failing
kind: story
status: draft
title: The 34 authored ESS scenarios that fail when executed are triaged and pass
relations:
- informed_by: story:ess-connector-vocabulary-model
revision: 1
---
## Context

The gate compiled authored ESS scenarios but never executed them (`crates/connectors-build/src/gate.rs`,
fixed in commit f42596816). Executed on the pinned `ess` interpreter, 34 of the 49 registered
authored scenarios fail. They are in the credential_evidence, idempotency, mutations, refresh and
sessions scenario files, and they predate story:ess-connector-vocabulary-model. Most expect an error
field (`state`, `reason`) that the ess/18 model never fills; the 7 mutations traces expect
different outcomes than the model gives (`subject-missing`, `wrong-state`, `aborted`). They are
listed in `AUTHORED_KNOWN_FAILING` so the gate fails on any new failure.

## Change

- Triage each of the 34: fix the scenario where the model is right, fix the model where the
  scenario is right, and remove it from `AUTHORED_KNOWN_FAILING`.

## Acceptance

`AUTHORED_KNOWN_FAILING` in `crates/connectors-build/src/gate.rs` is empty and the gate prints
`executed 49 authored scenarios, 49 passed`.
