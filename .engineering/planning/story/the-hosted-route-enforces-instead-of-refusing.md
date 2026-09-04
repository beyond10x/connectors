---
format: aep.planning-md/1
id: story:the-hosted-route-enforces-instead-of-refusing
kind: story
status: implemented
title: The hosted route enforces instead of refusing
refs:
- provider: legacy
  reference: S-046
relations:
- derived_from: epic:enforced-authority
scope:
- confidence: cited
  path: crates/server
- confidence: cited
  path: crates/service
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-046-the-hosted-route-enforces-instead-of-refusing.md:21`. **read**

- A caller with a valid route-family token but no admitting Grant is refused (403) on a mutation.
- A granted mutation with a demanded approval refuses without one, dispatches with one, and
  refuses the same approval's second use.
- The pre-existing read-only path is unchanged for callers without Grants covering effects.
- End-to-end test drives the full chain over the hosted HTTP surface.

## Context

Replace the two hosted fences (approval-reference refusal; ReadOnly+NotRequired re-describe gate)
with the real gates: every Invoke passes the GrantEvaluator, and descriptions demanding approval
additionally pass the approval gate. Effect-bearing hosted invocation becomes reachable — only
through proofs.

Source frontmatter: pillar Platform · areas [server, service] · design `../design/13-grant-evaluation-and-approval-redemption.md`. **read**

Source `note:` field, quoted: “server::hosted::HostedAuthority (GrantEvaluator + ApprovalGate over the one bound S-041 store) replaces both fences; described ReadOnly+NotRequired keeps the pre-existing policy path when no Grant admits; every enforcement 403 is one byte-identical axis-free body, replay distinct only in the journal; issued approval records live at approval.issued.<sha256(reference)> via the issue_approval bootstrap; the description carries no declared risk/effects/idempotency yet, so evaluation claims worst-case facts and exact allow exceptions are the admission shape until S-047 threads the proofs and facts onward”

## Status

`done` in the source. Quoted from `docs/stories/S-046-the-hosted-route-enforces-instead-of-refusing.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-046-the-hosted-route-enforces-instead-of-refusing.md`, which is not deleted and now names this artifact.

- First written 2026-08-23 · last touched 2026-08-23 · 2 revision(s)
- Legacy id `S-046`, recorded as the reference `legacy:S-046`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
