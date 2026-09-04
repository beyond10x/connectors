---
format: aep.planning-md/1
id: story:a-session-signal-carries-authority
kind: story
status: implemented
title: A session signal carries authority
refs:
- provider: legacy
  reference: S-049
relations:
- derived_from: epic:enforced-authority
scope:
- confidence: cited
  path: crates/domain
- confidence: cited
  path: crates/server
- confidence: cited
  path: crates/service
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-049-a-session-signal-carries-authority.md:22`. **read**

- An effect-bearing session signal is refused unless a Grant admits it; refusals are axis-free
  and journaled, matching the invoke path's semantics.
- The catalogue invariant family covers the session-signal seam so a future signal route cannot
  bypass admission silently.
- Hosted single-replica assumption for `ApprovalGate::recover` is either stated in design 13 or
  replaced with a fencing mechanism (S-047 review open question rides along here with the
  session-lifecycle work).

## Context

`SessionSignal` dispatches effect-bearing input (a keypress cannot be undone) behind scope and
operator-group policy only — no `GrantEvaluator`, no proof (S-046 and S-047 reviews, 2026-08-23).
It escaped the enforced-authority epic because it carries no operation ref, description, or
Connection: nothing a `GrantRequest` can name. Give session signals a describable authority —
either a described session-signal operation per provider or a session-scoped admission derived
from the Grant that admitted the session's creation — and bring the dispatch behind it.

Source frontmatter: pillar Platform · areas [server, domain, service] · design `../design/13-grant-evaluation-and-approval-redemption.md`. **read**

## Status

`done` in the source. Quoted from `docs/stories/S-049-a-session-signal-carries-authority.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-049-a-session-signal-carries-authority.md`, which is not deleted and now names this artifact.

- First written 2026-08-23 · last touched 2026-08-23 · 4 revision(s)
- Legacy id `S-049`, recorded as the reference `legacy:S-049`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
