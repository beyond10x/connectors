---
format: aep.planning-md/3
id: release-plan:connectors-v0710-grafana
kind: release-plan
status: draft
title: 'Release 0.71.0: Grafana through the catalog provider and wave B part 1'
tags:
- zero-open-issues
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Minor release 0.71.0, cut from the merge of PR 248 (`wave/grafana-catalog`) into `main`, which
already holds PR 247 (d1a12e29c6, wave B part 1 of epic:zero-open-issues-and-store-defects). The
operator decided on 2026-11-11 that this session cuts the versions and that pull requests merge
automatically once their gates pass.

- Grafana through the catalog provider: eight reads, five on the HTTP API and three through the
  data-source proxy (story:catalog-grafana-reads, story:catalog-grafana-datasource-proxy-reads).
- An `add_operation` amendment parameter may carry an anchored `pattern`; a served catalog
  instance takes `catalog.allowed_values`.
- The defect fixes of PR 247, listed in the CHANGELOG's 0.71.0 sections.

Minor, not patch: the Grafana selection, the amendment `pattern` and `catalog.allowed_values` are
additions. One change refuses a configuration that loaded before: a name that both a legacy
`[connectors]` entry and an `[instances]` entry derive is `instance_ambiguous`.

## Scope

| story | status |
|---|---|
| story:catalog-grafana-reads | implemented |
| story:catalog-grafana-datasource-proxy-reads | implemented |

PR 247's stories are recorded on `main` by that pull request.

## Evidence

- PR 248 head 6741d3bacc: Build documentation, custody on macOS and Windows, planning validate,
  release-built providers on macOS, provider programs on macOS, shared security and privacy: all
  success when read at 10:2x UTC; the repository gate and host and CLI on macOS and Windows were
  still running. The release commit's CI on the same pull request is the gate for the release.
- Grafana lanes on the unit commits a2ccf224ed and 20a34e1373: 0 failed (grafana 18, adversary 8,
  security 10 and 5, bundle_drift 1, connector_declarations 12, connectors-catalog 417).
- The vendored Grafana document is upstream `public/openapi3.json` at v13.3.1 minus one example
  member; both digests are in `adapters/grafana/upstream/README.md`.
- Pairs with connectors-definitions 0.7.0 (`connectors/grafana.yaml`), whose amendments need this
  release's `pattern` support.

## Not in this release

- Wave B part 2 on `wave/zero-issues-c` (PR 2): integration head not green.
- Draft story:catalog-json-answers-reencoded (filed from the Grafana adversary pass).
- No story moves with this record.
