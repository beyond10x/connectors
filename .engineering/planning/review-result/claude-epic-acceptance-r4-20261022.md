---
format: aep.planning-md/3
id: review-result:claude-epic-acceptance-r4-20261022
kind: review-result
status: active
title: 'Plan-critic acceptance, round 4: connectors with Claude'
summary: needs-revision, 7 warnings; addressed
relations:
- reviews: story:docs-first-run-path
- reviews: story:definition-reader-feed-extensions
- reviews: story:mcp-stdio-runtime-branch
- reviews: story:standing-write-approval-for-unattended-agents
revision: 1
---
# Plan-critic, acceptance lens, round 4: connectors with Claude

**Verdict: needs-revision. 7 warnings, no blocker. All are addressed in the revisions after round 4.**

| Story | Finding | Disposition |
|---|---|---|
| docs-first-run-path | "The seven wrong facts above are fixed" had no after-state | one line per fact with its after-state |
| docs-first-run-path | the clock block's destination was "`docs/`" | `docs/local-clock.md` |
| definition-reader-feed-extensions | the Confluence scenario named no fixture | `adapters/catalog/tests/feed/confluence/feed.json` and `providers/confluence-v1/operations.json` |
| mcp-stdio-runtime-branch | the whole-branch line had scenarios only for list, call, refusal | the two lease tests (`:123`, `:176`) are required in the jail; each of the branch's 28 test files re-lands and passes or is listed as dropped with a reason |
| mcp-stdio-runtime-branch | spec-first named no domain | `ess/domains/{cli,service_wire,transport}.yaml`; `task check` validates and conformance counts do not drop |
| standing-write | expiry, revocation and rate scenarios did not assert no request | `_with_no_request` on all three |
| standing-write | revocation read the same as no grant file | the scenario uses `grant-standing` and `revoke-standing <grant id>` before `not_after` |
