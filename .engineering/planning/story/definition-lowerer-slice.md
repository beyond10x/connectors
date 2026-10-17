---
format: aep.planning-md/3
id: story:definition-lowerer-slice
kind: story
status: implemented
title: Lower one connectors-definition/1 file into the catalog provider's bundle and selection
relations:
- implements: architecture-decision-record:provider-definitions-and-runtimes
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-16T17:52:03Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-16T17:52:04Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-17T04:51:03Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

`connectors-build definition` lowers one `connectors-definition/1` file from
`beyond10x/connectors-definitions` into what the catalog provider already runs: a provider bundle
plus its index, and a `connectors-catalog-operations/1` selection file. The catalog provider loads
the output unchanged and serves the selected operations.

This is the vertical slice of `architecture-decision-record:provider-definitions-and-runtimes`:
one provider (Jira, three reads), definition file to served operation, before the format grows.

## Input

The definition schema v1 is the ESS source `spec/domains/definition.yaml` in
`beyond10x/connectors-definitions` (branch `ess-validation`, PR
https://github.com/beyond10x/connectors-definitions/pull/1). Jira's file:

```yaml
schema: urn:beyond10x:connectors-definition:1
source:
  openapi: upstream/jira/jira-platform-v3.json.gz
  sha256: 655b4790a5c8543c81755c8250c6a8b8f58ee62712168a39d628d6d927549709
base_url: https://{site}.atlassian.net
config:
  site: {type: string}
auth:
  atlassian.basic: {scheme: basic}
operations:
  issue.get: {op: getIssue}
  issue.comments: {op: getComments}
  issues.search: {op: searchAndReconsileIssuesUsingJql}
```

## Acceptance

1. Lowering the Jira definition with the vendored source writes a bundle byte-identical to
   `adapters/catalog/generated/bundles/jira.bundle.json`, and a selection whose three entries carry
   the `id`, `operation_id` and `effect` of the matching entries in
   `adapters/catalog/providers/jira/operations.json`.
2. The catalog provider loads the lowered bundle and selection and serves `issue.get` against the
   existing Jira HTTPS fixture.
3. Each refusal is by name, with a test: a vendored source that does not hash to `source.sha256`; an
   `op` the bundle inventory lacks; a `base_url` variable no `config` field declares; a non-GET
   operation with no `effect`; more than one `auth` entry (not in v1); an unknown field.

## Out of scope

Paging, datasources, guards, bodies, OAuth, Swagger and Discovery sources, publishing, and the
`connectors definitions install` command.
