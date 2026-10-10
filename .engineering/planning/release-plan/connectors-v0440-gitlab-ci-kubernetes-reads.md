---
format: aep.planning-md/3
id: release-plan:connectors-v0440-gitlab-ci-kubernetes-reads
kind: release-plan
status: implemented
title: 'Release 0.44.0: GitLab CI and project writes, Kubernetes reads'
relations:
- serves: vision:independent-contract-adapters
- informed_by: decision-blocker:wave-20261010d-scope
revision: 3
transitions:
- {from: "draft", to: "active", at: "2026-10-10T13:38:19Z", actor: "human:timo", revision: 2}
- {from: "active", to: "implemented", at: "2026-10-10T14:50:01Z", actor: "human:timo", revision: 3}
---
## Outcome

Minor release 0.44.0: GitLab CI/CD and project writes through the catalog provider (pipeline
retry, cancel and create, environments, commit create, file update, branch create and delete,
project create), the source amendment kind `correct_media_type`, and Kubernetes single-object
reads, configured namespaces, events and rollout history.

Minor rather than patch: new selections change the GitLab descriptor and guide revision, and the
Kubernetes descriptor gains three operations. Existing configurations and adapter entries keep
working.

## Scope

| surface | change |
|---|---|
| `crates/connectors-catalog/src/amendment.rs`, `adapters/catalog/spec/ess/domains/amendment.yaml` | `correct_media_type` |
| `adapters/gitlab/upstream/openapi_v3.amendments.json` | three cited media-type corrections |
| `adapters/catalog/providers/gitlab/operations.json` | `pipeline.retry`, `pipeline.cancel`, `pipeline.create`, `environments.list`, `commit.create`, `file.update`, `branch.create`, `branch.delete`, `project.create` |
| `adapters/kubernetes` | `resources.get`, `namespaces.list`, `deployments.history`; kinds `replicasets`, `events`; ESS `domains/reads.yaml` |
| guides, parity page, README, CHANGELOG, website | documentation and version 0.44.0 |

Stories: `story:parity-gitlab-ci-writes`, `story:parity-gitlab-project-writes`,
`story:parity-kubernetes-reads`; decided in `decision-blocker:wave-20261010d-scope`.

## Evidence

- Wave PR https://github.com/beyond10x/connectors/pull/153, head `69dbc5fb8`, merged 2026-10-10
  as `41912e4b8`: repository gate, Build documentation, planning validate and Security and privacy
  passed.
- Package gates on `69dbc5fb8`: connectors-catalog, connectors-catalog-provider,
  connectors-build, connectors-kubernetes and connectors-docs 1082 tests passed, 0 failed, 34
  ignored; fmt and clippy clean; `connectors-docs generate --check` current; the catalog and
  Kubernetes models valid under ess 0.57.0; the Kubernetes CLI journey run once and passed.
- Adversary pass on the GitLab writes
  (`review-result:adversary-parity-gitlab-ci-project-writes-pass-1`); its finding fixed and its
  case kept as a test.
- Fixtures only; no live GitLab or Kubernetes call.

## Completion boundary

Not in this release: `story:parity-confluence-cql-search-v1-held` (whether Atlassian's REST v1
document may be vendored is undecided), `story:parity-jira-issue-writes`,
`story:parity-slack-message-writes`, `story:parity-homer-sip-reads`.
