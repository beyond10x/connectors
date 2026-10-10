---
format: aep.planning-md/3
id: release-plan:connectors-v0430-gitlab-search-prometheus
kind: release-plan
status: implemented
title: 'Release 0.43.0: GitLab code search, Prometheus, GitLab tags and releases'
relations:
- serves: vision:independent-contract-adapters
- informed_by: decision-blocker:wave-20261010c-scope
revision: 3
transitions:
- {from: "draft", to: "active", at: "2026-10-10T09:10:52Z", actor: "human:timo", revision: 2}
- {from: "active", to: "implemented", at: "2026-10-10T12:13:51Z", actor: "human:timo", revision: 3}
---
## Outcome

Minor release 0.43.0: GitLab code search, a native Prometheus adapter (direct and through
Grafana), and GitLab tags and releases. Two catalog capabilities make the search selectable (a
cited path correction in a source amendment; a string value bound in a selection), and a third
proves a delete (`postflight.absent`).

Minor rather than patch: a new adapter, new selections that change the GitLab descriptor and guide
revision, and Rust API changes to `Bound` and `Postflight`. Existing configurations and adapter
entries keep working.

## Scope

| surface | change |
|---|---|
| `crates/connectors-catalog/src/amendment.rs` | `correct_path` in source amendments |
| `adapters/catalog/src/lib.rs` | `values` bounds, `postflight.absent` |
| `adapters/catalog/spec/ess/domains/amendment.yaml`, `selection.yaml`, `guard.yaml` | the catalog amendment, selection and guard model |
| `adapters/gitlab/upstream/openapi_v3.amendments.json` | the cited correction of the project search path |
| `adapters/catalog/providers/gitlab/operations.json` | `search.blobs`, `tag.get`, `tag.create`, `tag.delete`, `release.get`, `release.links`, `release.create`, `release.update` |
| `adapters/prometheus` | the new adapter, its ESS model and contract §11 |
| guides, parity page, README, CHANGELOG, website | documentation and version 0.43.0 |

Stories: `story:catalog-path-correction-and-value-bound`, `story:parity-prometheus-query`,
`story:parity-gitlab-tags-releases`, and with them `story:parity-gitlab-repository-reads`
completes; decided in `decision-blocker:wave-20261010c-scope`.

## Evidence

- Wave PR https://github.com/beyond10x/connectors/pull/151, head `a5aa181c6`, merged 2026-10-10
  as `5bf160ec2`: repository gate (24m10s), Build documentation, planning validate and Security
  and privacy passed.
- Package gates on `a5aa181c6`: connectors-catalog, connectors-catalog-provider,
  connectors-build, connectors-prometheus and connectors-docs 1045 tests passed, 0 failed, 27
  ignored; fmt and clippy clean; the catalog and Prometheus models valid under ess 0.57.0; the
  two Prometheus CLI journeys run and passed.
- Adversary passes on the two catalog units (`review-result:adversary-catalog-path-correction-and-value-bound-pass-1`,
  `review-result:adversary-parity-gitlab-tags-releases-pass-1`); findings fixed and their cases
  kept as tests.
- Fixtures only; no live GitLab, Prometheus or Grafana call.

## Completion boundary

Not in this release: `story:parity-jira-issue-writes`, `story:parity-slack-message-writes`,
`story:catalog-guard-typed-comparison`, `story:metadata-conformance-emit-manifest-format`.
Prometheus limits: no tenant header, no `timeout` parameter, bearer profile only.
