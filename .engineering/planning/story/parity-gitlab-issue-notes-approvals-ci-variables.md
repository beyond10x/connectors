---
format: aep.planning-md/3
id: story:parity-gitlab-issue-notes-approvals-ci-variables
kind: story
status: draft
title: GitLab issue notes, approvals and CI variables
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

The GitLab operations the old plugins declared that a development team uses on issues, approvals
and CI variables are served by the catalog provider: `issue.notes.list` (read),
`issue.note.create`, `issue.update`, `merge_request.approve`, `merge_request.discussion.create`,
`ci.variable.create`, `ci.variable.update` and `ci.variable.delete`. All eight had 0 calls in the
baseline (2026-09-09 to 2026-10-07); they are held for first demand (phase P4, decision D4 of the
epic's plan), scheduled when `connectors operations usage` reports a `not_found` refusal naming
one of them. Every endpoint exists in the pinned GitLab document
(`adapters/gitlab/upstream/openapi_v3.yaml`); none needs a new adapter.

The other 11 missing operations of the old plugins (release delete and release links, snippets,
`branch.delete_merged`, changelog add and generate, `repository.archive`, `mr.diff.lines`) stay
unplanned until used, per the page's rule.

## Surface

Selections in `adapters/catalog/providers/gitlab/operations.json`, admitted in `connector.json`,
with the guards the merge-request writes use (`docs/local-gitlab-merge.md`): the issue update has
a preflight on the issue's `updated_at` and refuses `conflict`; the approval has a postflight read
of the approval state; the variable writes have a preflight read of the variable by key and the
delete is admitted only under an approval naming the key; the note and discussion writes have a
postflight read by id.

## Acceptance

- Spec first: new guard shapes, if any, are modelled in
  `adapters/catalog/spec/ess/domains/selection.yaml` and validated with the pinned `ess`.
- Each operation answers through `connectors operations invoke` under an approval file against a
  recorded fixture in the pinned document's shapes; each guard's refusal is a test.
- The parity page rows move to covered with the Connectors operations named; the GitLab guide
  (`docs/local-catalog-provider.md`) gains the commands run in the tree.

## Scope

Primary: `adapters/catalog/providers/gitlab/operations.json`,
`adapters/catalog/providers/gitlab/connector.json`, `adapters/catalog/tests/gitlab*.rs`,
`docs/local-catalog-provider.md`, `website/docs/connectors/gitlab.md`. Collides with any other
GitLab selection story (`story:catalog-gitlab-search` on the wave branch).
