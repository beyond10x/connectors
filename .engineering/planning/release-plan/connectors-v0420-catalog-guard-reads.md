---
format: aep.planning-md/3
id: release-plan:connectors-v0420-catalog-guard-reads
kind: release-plan
status: active
title: 'Release 0.42.0: catalog guard reads and fixed body values; Jira transition run, GitLab auto-merge, reopen and merge-request reads'
relations:
- serves: vision:independent-contract-adapters
- informed_by: decision-blocker:wave-20261010b-scope
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-10T03:47:14Z", actor: "human:timo", revision: 2}
---
## Outcome

Minor release 0.42.0: two catalog engine changes and the selections they unblock. A guard may
read more than once before a write and once after it; a selection may fix a body member to one
value, and a postflight may accept one of several observations. With them, the Jira transition
run, GitLab merge-when-pipeline-succeeds and reopen, and GitLab merge-request diffs and the
discussion list.

Minor rather than patch: new selections change the Jira and GitLab descriptors and guide
revisions, and the selection format gains four optional members. Existing configurations and
adapter entries keep working.

## Scope

| surface | change |
|---|---|
| `adapters/catalog/src/lib.rs`, `adapters/catalog/src/local.rs` | `further_preflights`, `postflight.read`, `postflight.any_of`, `body_fixed` |
| `adapters/catalog/spec/ess/domains/guard.yaml`, `selection.yaml` | the catalog guard and selection model |
| `adapters/catalog/providers/jira/operations.json` | `issue.transition.run` |
| `adapters/catalog/providers/gitlab/operations.json` | `merge_request.auto_merge`, `merge_request.reopen`, `merge_request.diffs`, `merge_request.discussions` |
| guides, parity page, README, CHANGELOG, status pages | documentation and version 0.42.0 |

Stories: `story:catalog-guard-postflight-read`, `story:catalog-selection-fixed-body-value`,
`story:parity-gitlab-mr-reads`, and with them `story:parity-jira-transitions` and
`story:parity-gitlab-mr-writes` complete; decided in `decision-blocker:wave-20261010b-scope`.

## Evidence

- Wave PR https://github.com/beyond10x/connectors/pull/149, head `557551493`, merged 2026-10-10
  as `6522db099`: repository gate (31m57s), Build documentation, planning validate and Security
  and privacy passed.
- Package gates on `557551493`: connectors-catalog-provider, connectors-build and
  connectors-docs 788 tests passed, 0 failed, 25 ignored; fmt and clippy clean; the catalog
  model valid under ess 0.57.0.
- One adversary pass on the two engine units; its three findings fixed in `3b09b57d1` and its
  cases kept as regression tests.
- Fixtures only, through the engine and the owned provider process; no live Jira or GitLab
  call.

## Completion boundary

Not in this release: `story:catalog-path-correction-and-value-bound` (GitLab code search, so
`story:parity-gitlab-repository-reads` stays active), `story:parity-jira-issue-writes`,
`story:parity-slack-message-writes`.
