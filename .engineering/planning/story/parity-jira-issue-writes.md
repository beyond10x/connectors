---
format: aep.planning-md/3
id: story:parity-jira-issue-writes
kind: story
status: active
title: Jira issue create, edit, comment, link and delete
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
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-18T22:42:35Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-18T22:42:35Z", actor: "human:timo", revision: 6}
---
## Outcome

Jira issue edit, link and delete through the catalog provider: `editIssue` as `issue.edit`,
`linkIssues` as `issue.link.add` and `deleteIssue` as `issue.delete`, each `effect: write` under an
issued approval. The second half of parity unit U06 of `docs/fluxplane-plugin-parity.md`:
`jira.issue.link.add` (40 calls), `jira.issue.edit` (26) and `jira.issue.delete` (3), 69 of the
unit's 326 calls. Create and comment are story:parity-jira-issue-create-and-comment, which lands
first on the same selection file.

## Surface

`adapters/catalog/providers/jira/operations.json` gains three selections;
`adapters/catalog/providers/jira/connector.json` admits them. Guards, in the shape the Zendesk
ticket writes use (`docs/catalog-zendesk.md`, "Writes"): `issue.edit` has a preflight `issue.get`
on the caller's `updated` stamp and is refused as `conflict` with nothing sent when the issue
changed; `issue.link.add` has a postflight read of the issue's links that names the new link;
`issue.delete` has a preflight `issue.get` confirming the key and the caller's `updated` stamp,
is admitted only under an approval whose scope names the key, and reports `partial` when the
read-back after a `204` still finds the issue. Text fields of `issue.edit` use the plain-text
wrapper of story:parity-jira-issue-create-and-comment.

## Acceptance

Amended 2026-10-19 by decisions D1 and D8 of the epic's plan of that date: a description is an
Atlassian Document Format document the caller sends as given, and `issue.link.add` has no
read-after (Jira answers `201` with no body). No new engine member was needed, so nothing was
modelled in `selection.yaml` first. The approval itself is the host's approval coordinator, bound to
the exact input of each write and tested there; the provider tests run with a spent approval.

- Each write runs through the catalog provider against a fixture in the pinned v3 document's
  shapes: an edit reads the `updated` stamp, puts once and reads the issue after; a link reads both
  issues and posts once; a delete is proved by a `404` on the read after it, and an issue still
  found after it answers `unknown` (`adapters/catalog/tests/jira_issue_writes.rs`).
- A changed `updated` stamp refuses `issue.edit` and `issue.delete` as `conflict` and the fixture
  receives no write; Jira's `409` to an edit is `conflict`, never retried
  (`a_stale_stamp_refuses_edit_and_delete_as_conflict_with_nothing_written`,
  `jira_409_to_an_edit_is_conflict_and_never_retried`).
- The parity page rows move to covered with the Connectors operations named; the "Issue writes"
  section of `docs/catalog-jira.md` covers the five writes.

## Scope

Primary: `adapters/catalog/providers/jira/operations.json`,
`adapters/catalog/providers/jira/connector.json`, `adapters/catalog/tests/jira.rs`,
`docs/catalog-jira.md`, `website/docs/connectors/jira.md`. Collides with
story:parity-jira-issue-create-and-comment (runs before) and
story:parity-jira-create-meta-per-type (same file; either order after the first).
