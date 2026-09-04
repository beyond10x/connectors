---
format: aep.planning-md/1
id: story:an-approval-is-redeemed-exactly-once
kind: story
status: implemented
title: An approval is redeemed exactly once
refs:
- provider: legacy
  reference: S-045
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

Verbatim from `docs/stories/S-045-an-approval-is-redeemed-exactly-once.md:23`. **read**

- Redemption is atomic under concurrent presentation: exactly one of N concurrent identical
  requests proceeds (test with real concurrency, not sequential simulation).
- A mismatched operation, Connection, digest or expired record refuses without naming the axis.
- Attempted audit exists for every dispatch attempt, including refusals and crashes between
  redemption and terminal write (proven by a kill-point test or documented crash-recovery scan).

## Context

Implement the approval verifier and redemption store: an externally issued approval record
(issuer, subject, operation, Connection, canonical input digest, expiry) is verified against the
invocation and atomically redeemed — the attempted-audit row and the redemption land in one
transaction before dispatch; the terminal outcome row follows after. A second presentation of the
same reference refuses and audits as replay.

Source frontmatter: pillar Platform · areas [domain, service, state, audit] · priority 2 · design `../design/13-grant-evaluation-and-approval-redemption.md`. **read**

Source `note:` field, quoted: “ApprovalGate in domain::approval — axis-free verification; the one-time claim is a bounded append to approval.redemption.<sha256(reference)> whose payload IS the attempted-audit row; replay is its own journal kind; ApprovalGate::recover is the documented startup crash scan. SQLite concurrency/replay/crash evidence in state-sqlite/tests/approval_gate.rs; S-046 wires the hosted route”

## Status

`done` in the source. Quoted from `docs/stories/S-045-an-approval-is-redeemed-exactly-once.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-045-an-approval-is-redeemed-exactly-once.md`, which is not deleted and now names this artifact.

- First written 2026-08-23 · last touched 2026-08-23 · 2 revision(s)
- Legacy id `S-045`, recorded as the reference `legacy:S-045`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
