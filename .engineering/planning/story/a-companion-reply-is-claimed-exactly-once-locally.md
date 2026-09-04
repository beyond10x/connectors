---
format: aep.planning-md/1
id: story:a-companion-reply-is-claimed-exactly-once-locally
kind: story
status: implemented
title: A companion reply is claimed exactly once, locally too
refs:
- provider: legacy
  reference: S-048
relations:
- derived_from: epic:enforced-authority
revision: 5
---
## Acceptance

Verbatim from `docs/stories/S-048-a-companion-reply-is-claimed-exactly-once-locally.md:24`. **read**

- On the local placement, two presentations of the same companion event produce exactly one
  outward Slack reply; the second refuses and is journaled, proven by a real-concurrency test.
- Catalog invariant rule 15 stays green without weakening its empty-by-default posture.
- The SIP `approval_evidence_ref` config field, which no code consumes since S-047, is retired
  from `connectors-config` (`personal.rs`), the examples, and the connectors-cli README — or its
  retention is justified in the config docs.

## Context

S-047 deleted the Slack backend's `ReplyClaimStore` with the approval-presence check it rode on,
so on the local placement nothing enforces one reply per companion mention any more: a retried
invoke can post the same Slack reply twice (S-047 review, 2026-08-23). Restore the exactly-once
claim as an honest local mechanism — a durable one-time claim keyed on the triggering event,
made at the dispatch seam rather than by the Integration re-reading `approval_evidence_ref`
(which catalog invariant rule 15 now forbids; its named allowlist
`APPROVAL_EVIDENCE_AUDIT_CORRELATION_USES` is the sanctioned escape hatch if correlation by that
field turns out to be the right shape).

Source frontmatter: pillar Platform · areas [integrations, state, config] · design `../design/13-grant-evaluation-and-approval-redemption.md`. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-048-a-companion-reply-is-claimed-exactly-once-locally.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-048-a-companion-reply-is-claimed-exactly-once-locally.md`, which is not deleted and now names this artifact.

- First written 2026-08-23 · last touched 2026-08-23 · 3 revision(s)
- Legacy id `S-048`, recorded as the reference `legacy:S-048`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
