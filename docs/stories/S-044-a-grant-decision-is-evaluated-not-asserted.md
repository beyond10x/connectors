---
id: S-044
title: "A Grant decision is evaluated, not asserted"
pillar: Platform
status: done
priority: 2
design: ../design/13-grant-evaluation-and-approval-redemption.md
epic: enforced-authority
areas: [domain, service, state]
note: "GrantEvaluator + revisioned Grant records land in domain over the S-041 port; the cfg(test) decision builder is deleted, so the evaluator is the only constructor; grant_conformance runs on memory, SQLite (memory+file) and #[ignore]d live PostgreSQL; hosted wiring stays with S-046"
---

# A Grant decision is evaluated, not asserted

## Goal

Implement the `GrantEvaluator` over the S-041 state port: revisioned tenant Grants, selector
semantics (risk ceiling, effects subset, idempotency class), explicit exceptions with
deny > allow > predicate, inbound events as closed sets. Success is a `GrantDecision` proof —
private fields, module-sealed, bound to issuer, tenant, sub, act, operation ref, Connection ref,
catalog generation, description lease, grant revision, canonical input digest, decision, expiry
and a one-time id.

## Acceptance

- No store bound → 503; empty store or nothing admitting → 403; no refusal names the axis.
- Deny beats allow beats predicate, proven by tests including an allow+deny collision.
- A `GrantDecision` cannot be constructed from any other crate (compile-fail test or doc-tested
  seal), and an expired decision refuses at use.
- SQLite and hosted-PostgreSQL store backends pass one shared conformance exercise.

## Superseded by

`story:a-grant-decision-is-evaluated-not-asserted` in the AEP planning store, at
`.engineering/planning/story/a-grant-decision-is-evaluated-not-asserted.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
