---
format: aep.planning-md/3
id: story:parity-jira-create-meta-per-type
kind: story
status: implemented
title: The create-metadata fields of one Jira issue type
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:parity-jira-issue-create-and-comment
scope:
- confidence: cited
  path: adapters/catalog/providers/jira/connector.json
- confidence: cited
  path: adapters/catalog/providers/jira/operations.json
- confidence: cited
  path: adapters/catalog/tests/connector_declarations.rs
- confidence: cited
  path: adapters/catalog/tests/jira.rs
- confidence: inferred
  path: adapters/catalog/tests/jira_issue_writes.rs
- confidence: cited
  path: crates/connectors-catalog/tests/fixtures/definitions/connectors/jira.yaml
- confidence: cited
  path: crates/connectors-docs/src/status.rs
- confidence: cited
  path: docs/catalog-jira.md
- confidence: cited
  path: website/docs/connectors/jira.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T22:42:35Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-18T22:42:36Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-19T05:54:45Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":2,"review_outcome":1}}}
---
## Outcome

The create-metadata fields of one Jira issue type are readable: `getCreateIssueMetaIssueTypeId`
selected as `issue.create_meta.fields`, so `jira.issue.create_meta` (5 calls) moves from partial
to covered. Today `issue.create_meta` (`adapters/catalog/providers/jira/operations.json:15`) lists
a project's issue types only; story:parity-jira-issue-reads recorded the per-type read as a known
limit.

## Surface

One read selection in `adapters/catalog/providers/jira/operations.json`, admitted in
`connector.json`, with the project key and issue type id as required parameters and the page
bounds the catalog engine requires.

## Acceptance

- Spec first: the selection is modelled in `adapters/catalog/spec/ess/domains/selection.yaml` if a
  new member is needed (none is expected), validated with the pinned `ess`.
- `connectors operations invoke --connector jira --instance <i> --operation
  issue.create_meta.fields` with a project key and an issue type id answers the field list from a
  recorded fixture; a missing issue type id is refused as `invalid_input` naming it.
- The parity page row moves to covered and the Jira guide names the read.

## Scope

Primary: `adapters/catalog/providers/jira/operations.json`,
`adapters/catalog/providers/jira/connector.json`, `adapters/catalog/tests/jira.rs`,
`docs/catalog-jira.md`. Collides with the two Jira write stories on the same file; runs after
story:parity-jira-issue-create-and-comment.
