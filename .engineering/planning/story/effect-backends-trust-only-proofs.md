---
format: aep.planning-md/1
id: story:effect-backends-trust-only-proofs
kind: story
status: implemented
title: Effect backends trust only proofs
refs:
- provider: legacy
  reference: S-047
relations:
- derived_from: epic:enforced-authority
revision: 5
---
## Acceptance

Verbatim from `docs/stories/S-047-effect-backends-trust-only-proofs.md:21`. **read**

- No integration reads `approval_evidence_ref` for admission; the field reaches backends only as
  audit correlation.
- One parameterised invariant in `catalog_invariants.rs` (house rule: one rule, all providers)
  fails if a future effect operation bypasses the proof chain.
- The workspace gate is green.

## Context

Delete the per-integration approval-presence checks (gitlab, kubernetes, slack, sip, jira,
monitoring) — the proof chain upstream is the authority — and add the catalogue-wide invariant
test that no effect-bearing backend code path is reachable without an `AdmittedOperation` built
from proofs.

Source frontmatter: pillar Platform · areas [integrations, testing] · design `../design/13-grant-evaluation-and-approval-redemption.md`. **read**

Source `note:` field, quoted: “every Integration's local approval-presence check is deleted (kubernetes hosted+local, monitoring, jira, gitlab, sip, slack, b10x) — the S-043..S-046 proof chain is the only admission authority; InvokeAdmission::Proven now carries the AdmittedOperation and hosted dispatch consumes it by value; catalog_invariants rule 15 scans every integration crate for approval_evidence_ref reads with an empty named audit-correlation allowlist; Slack's event-authorized companion reply (ReplyClaimStore) went with its check — an event that should authorize a reply must be issued as an approval record; ApprovalGate::recover is wired into HostedRuntime startup; GrantEffect::ALL makes the worst-case facts exhaustive by construction”

## Status

`done` in the source. Quoted from `docs/stories/S-047-effect-backends-trust-only-proofs.md:5`: `status: done`. **read**

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

Migrated from `docs/stories/S-047-effect-backends-trust-only-proofs.md`, which is not deleted and now names this artifact.

- First written 2026-08-23 · last touched 2026-08-23 · 2 revision(s)
- Legacy id `S-047`, recorded as the reference `legacy:S-047`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
