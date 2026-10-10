---
format: aep.planning-md/3
id: release-plan:connectors-v0410-parity-gitlab-jira
kind: release-plan
status: active
title: 'Release 0.41.0: Jira transitions, GitLab merge-request writes and repository reads, ESS 0.57.0'
relations:
- serves: vision:independent-contract-adapters
- informed_by: decision-blocker:wave-20261010a-scope
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-10T01:06:01Z", actor: "human:timo", revision: 2}
---
## Outcome

Minor release 0.41.0: the next parity units through the catalog provider (the Jira transitions
read, GitLab merge-request notes, replies and a guarded resolve, GitLab repository tree, commit,
diff and branch reads), typed and required body keys in the selection format, and ESS 0.57.0.

Minor rather than patch: new selections change the Jira and GitLab descriptors and guide
revisions, and the selection format gains two optional fields. Existing configurations and
adapter entries keep working.

## Scope

| surface | change |
|---|---|
| `adapters/catalog/providers/jira/operations.json` | `issue.transitions` |
| `adapters/catalog/providers/gitlab/operations.json` | `merge_request.note.create`, `merge_request.discussion.reply`, `merge_request.discussion.get`, `merge_request.discussion.resolve`, `repository.tree`, `commit.get`, `commit.diff`, `branches.list` |
| `adapters/catalog/src/lib.rs` | `body_types`, `body_required` |
| `Cargo.toml`, `Cargo.lock`, `crates/connectors-spec/toolchain.json`, `apps/connectors-cli-contract/src` | ESS 0.57.0 |
| guides, parity page, README, CHANGELOG, status pages | documentation and version 0.41.0 |

Stories: `story:parity-jira-transitions`, `story:parity-gitlab-mr-writes` and
`story:parity-gitlab-repository-reads`, each in part (they stay active); decided in
`decision-blocker:wave-20261010a-scope`.

## Evidence

- Wave PR https://github.com/beyond10x/connectors/pull/147, head `6b6b98d2a`, merged 2026-10-10
  as `7350e9dea`: repository gate (32m1s), Build documentation, planning validate and Security and
  privacy passed.
- Package gates on `6b6b98d2a`: connectors-catalog-provider 492, connectors-docs 34,
  connectors-build 231, connectors-spec 17, connectors 128, connectors-conformance 14 tests passed;
  fmt and clippy clean.
- Two adversary passes on the GitLab writes; every finding fixed, pinned as documented behaviour,
  or drafted as `story:catalog-guard-typed-comparison`.
- Fixtures only; no live Jira or GitLab call.

## Completion boundary

Not in this release, drafted: `story:catalog-guard-postflight-read` (Jira run),
`story:catalog-selection-fixed-body-value` (GitLab auto-merge and reopen),
`story:catalog-path-correction-and-value-bound` (GitLab code search),
`story:metadata-conformance-emit-manifest-format`.
