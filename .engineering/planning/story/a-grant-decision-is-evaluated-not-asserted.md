---
format: aep.planning-md/1
id: story:a-grant-decision-is-evaluated-not-asserted
kind: story
status: implemented
title: A Grant decision is evaluated, not asserted
refs:
- provider: legacy
  reference: S-044
relations:
- derived_from: epic:enforced-authority
scope:
- confidence: cited
  path: crates/domain
- confidence: cited
  path: crates/service
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-044-a-grant-decision-is-evaluated-not-asserted.md:24`. **read**

- No store bound → 503; empty store or nothing admitting → 403; no refusal names the axis.
- Deny beats allow beats predicate, proven by tests including an allow+deny collision.
- A `GrantDecision` cannot be constructed from any other crate (compile-fail test or doc-tested
  seal), and an expired decision refuses at use.
- SQLite and hosted-PostgreSQL store backends pass one shared conformance exercise.

## Context

Implement the `GrantEvaluator` over the S-041 state port: revisioned tenant Grants, selector
semantics (risk ceiling, effects subset, idempotency class), explicit exceptions with
deny > allow > predicate, inbound events as closed sets. Success is a `GrantDecision` proof —
private fields, module-sealed, bound to issuer, tenant, sub, act, operation ref, Connection ref,
catalog generation, description lease, grant revision, canonical input digest, decision, expiry
and a one-time id.

Source frontmatter: pillar Platform · areas [domain, service, state] · priority 2 · design `../design/13-grant-evaluation-and-approval-redemption.md`. **read**

Source `note:` field, quoted: “GrantEvaluator + revisioned Grant records land in domain over the S-041 port; the cfg(test) decision builder is deleted, so the evaluator is the only constructor; grant_conformance runs on memory, SQLite (memory+file) and #[ignore]d live PostgreSQL; hosted wiring stays with S-046”

## Status

`done` in the source. Quoted from `docs/stories/S-044-a-grant-decision-is-evaluated-not-asserted.md:5`: `status: done`. **read**

This artifact reached `implemented` with `aep artifact move --evidence test_result=1`. The journal
records that move as resting on an **assertion**, not on a run this migration observed. The flag is
what the CLI provides for evidence that lives outside the store.

What was asserted, and where it came from:

- The source records `status: done` at the line quoted above. **read**
- `bash scripts/gate.sh` was green at commit `a48030b` on 2026-09-04 — exit 0, 136 `test result: ok`
  lines across 11 workspaces. **read**, from `~/.cache/connectors-gate/gate2.log`

No per-story run was attributed to this story. The gate is a repository-wide fact, and reading it as
proof of one story's acceptance would be an inference this record does not make.

## Provenance

Migrated from `docs/stories/S-044-a-grant-decision-is-evaluated-not-asserted.md`, which is not deleted and now names this artifact.

- First written 2026-08-23 · last touched 2026-08-23 · 2 revision(s)
- Legacy id `S-044`, recorded as the reference `legacy:S-044`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
