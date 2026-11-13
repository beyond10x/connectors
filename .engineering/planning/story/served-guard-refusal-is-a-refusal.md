---
format: aep.planning-md/3
id: story:served-guard-refusal-is-a-refusal
kind: story
status: draft
title: A served write refused before dispatch answers a refusal, not outcome_unknown
revision: 1
---
## Defect

Found by a served-host test of a Slack bot instance (2026-11-12), pre-existing. A served write whose
guard (a preflight read such as `auth.test`) answers not-ok sends nothing upstream, yet the served
host answers `502 outcome_unknown`, classification `unknown`, message "the write was dispatched and
its outcome is unknown". Every failed preparation becomes `WriteOutcome::Unknown`
(`adapters/catalog/src/served.rs:853`, documented as intended at `:838`), which
`crates/connectors-host/src/server/v1alpha2.rs:587-591` turns into `outcome_unknown`. A client that
follows the contract must then treat the write as possibly applied and must not retry under the same
idempotency key, though nothing was sent.

## Acceptance

- A served write refused before dispatch (guard not ok, guard read failed, input refused) answers a
  refusal (`4xx`, classification `refused`) naming its cause, and the fake upstream records no write.
- The ignored test `a_served_write_whose_guard_answers_not_ok_is_refused_not_unknown` in
  `adapters/catalog/tests/served.rs` runs un-ignored and passes.
