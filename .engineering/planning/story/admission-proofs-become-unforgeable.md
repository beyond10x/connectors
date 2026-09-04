---
format: aep.planning-md/1
id: story:admission-proofs-become-unforgeable
kind: story
status: implemented
title: Admission proofs become unforgeable
refs:
- provider: legacy
  reference: S-043
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

Verbatim from `docs/stories/S-043-admission-proofs-become-unforgeable.md:23`. **read**

- `from_grant_decision` no longer exists; grepping for it finds only history.
- `AdmittedOperation` has no public constructor; construction sites outside `crates/domain` use
  the named local-owner path, and its rustdoc states what it asserts and where it must not be used
  (hosted request handling).
- The workspace gate is green with no test weakened.

## Context

Remove `AdmittedOperation::from_grant_decision` — a public constructor over caller-supplied
strings — and make `AdmittedOperation` reachable only through two honestly named, sealed paths:
a `GrantDecision` proof (hosted; constructor lands with S-044, a module-private placeholder seals
the type now) and a local-owner admission for personal placements speaking over the owner's own
socket. Every current caller moves to the local-owner path explicitly; behavior does not change.

Source frontmatter: pillar Platform · areas [domain, service, integrations] · priority 1 · design `../design/13-grant-evaluation-and-approval-redemption.md`. **read**

Source `note:` field, quoted: “GrantDecision sealed in domain::evaluator (S-044 owns the production constructor); all 22 call sites moved to for_local_owner; the hosted-registry backends that reach it on read-only invocations are flagged for S-046”

## Status

`done` in the source. Quoted from `docs/stories/S-043-admission-proofs-become-unforgeable.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-043-admission-proofs-become-unforgeable.md`, which is not deleted and now names this artifact.

- First written 2026-08-23 · last touched 2026-08-23 · 2 revision(s)
- Legacy id `S-043`, recorded as the reference `legacy:S-043`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
