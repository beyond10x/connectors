---
format: aep.planning-md/3
id: story:parity-jira-issue-create-and-comment
kind: story
status: active
title: Jira issue create and comment add
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- depends_on: story:definition-format-write-members
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
- {from: "draft", to: "proposed", at: "2026-10-18T22:42:34Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-18T22:42:34Z", actor: "human:timo", revision: 5}
---
## Outcome

Jira issue create and comment add through the catalog provider: `createIssue` as `issue.create`
and `addComment` as `issue.comment.add`, each `effect: write` under an issued approval. The first
half of parity unit U06 of `docs/fluxplane-plugin-parity.md`: `jira.issue.create` (115 calls) and
`jira.issue.comment.add` (141 calls, plus 1 mapped `jira.issue.comment.create`), 257 of the unit's
326 calls.

## Surface

`adapters/catalog/providers/jira/operations.json` (9 ids today, one write `issue.transition.run`
at line 23) gains two selections; `adapters/catalog/providers/jira/connector.json` admits them; the
pinned Jira Cloud OpenAPI source under `adapters/atlassian/` is unchanged.

A text body (the comment's body, the issue's description) is a plain-text input the selection
wraps in a fixed Atlassian Document Format document (one `doc` with one `paragraph` of one `text`
node), using the nested closed write bodies, fixed values and `body_keys` pointers the catalog
engine admits since 0.54.0 (`contracts/catalog/v1alpha1/semantics.md`, "Writes"). Markdown is not
converted (decision D1 of the epic's plan). If the engine cannot place caller text inside a fixed
nested document, that capability is the first step of this story, modelled in
`adapters/catalog/spec/ess/domains/selection.yaml` before the selections are written.

Guards: `issue.create` has a postflight `issue.get` that confirms the key returned and the fields
sent (project, issue type, summary) are present on the created issue; a dropped field is reported
as `partial` with the field named, not as success. `issue.comment.add` has a postflight read of the
comment by id. Neither needs a preflight: both append.

## Acceptance

Amended 2026-10-19 by decision D1 of the epic's plan of that date: a Jira text body is an Atlassian
Document Format document the caller sends as given; the plain-text wrapper, its `invalid_input`
refusal and the read-back comparison of the description moved to story:plain-text-comment-bodies.
No new engine member was needed, so nothing was modelled in `selection.yaml` first.

- `issue.create` with project key, issue type id, summary and an ADF description, run through the
  catalog provider under a spent approval against a fixture in the pinned v3 document's shapes,
  reads the project first, posts the fields as given and answers the key Jira returns
  (`adapters/catalog/tests/jira_issue_writes.rs`,
  `an_approved_create_reads_the_project_and_posts_the_fields_as_given`).
- `issue.comment.add` reads the issue first and posts the document with `body.type` `doc` and
  `body.version` `1` fixed by the selection; the caller cannot override either
  (`an_approved_comment_reads_the_issue_and_posts_the_fixed_document`).
- A missing or empty required key, or a key the selection does not admit, is refused before any
  request (`malformed_inputs_are_refused_before_any_request`).
- The parity page rows `jira.issue.create` and `jira.issue.comment.add` move to covered with the
  Connectors operations named; `docs/catalog-jira.md` has an "Issue writes" section.

## Scope

Primary: `adapters/catalog/providers/jira/operations.json`,
`adapters/catalog/providers/jira/connector.json`, `adapters/catalog/spec/ess/domains/selection.yaml`
(only if the wrapper needs a new member), `adapters/catalog/tests/jira.rs`, `docs/catalog-jira.md`,
`website/docs/connectors/jira.md`. Collides with story:parity-jira-issue-writes and
story:parity-jira-create-meta-per-type on the same selection file: they run after this story.
