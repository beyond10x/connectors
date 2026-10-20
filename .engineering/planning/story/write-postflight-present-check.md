---
format: aep.planning-md/3
id: story:write-postflight-present-check
kind: story
status: active
title: A write's postflight can require a member to be present, and Jira create and comment use it
summary: issue.create and issue.comment.add report applied on a 2xx that names no id
relations:
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/catalog/providers/jira/operations.json
- confidence: cited
  path: adapters/catalog/spec/ess/domains/guard.yaml
- confidence: cited
  path: adapters/catalog/src/lib.rs
- confidence: cited
  path: adapters/catalog/tests/connector_declarations.rs
- confidence: cited
  path: adapters/catalog/tests/gateway_prefix.rs
- confidence: cited
  path: adapters/catalog/tests/jira_issue_writes.rs
- confidence: cited
  path: adapters/catalog/tests/jira_write_postflight_present.rs
- confidence: cited
  path: adapters/catalog/tests/write_postflight_present_adversary.rs
- confidence: cited
  path: adapters/catalog/tests/write_postflight_present_adversary_2.rs
- confidence: cited
  path: contracts/catalog/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-catalog/src/definition.rs
- confidence: cited
  path: crates/connectors-catalog/src/import.rs
- confidence: cited
  path: crates/connectors-catalog/tests/definition_present_adversary.rs
- confidence: cited
  path: crates/connectors-catalog/tests/fixtures/definitions/connectors/jira.yaml
- confidence: cited
  path: docs/catalog-jira.md
- confidence: cited
  path: docs/local-catalog-provider.md
- confidence: cited
  path: website/docs/guides/author-a-definition.md
- confidence: cited
  path: website/docs/reference/contracts/catalog.md
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-19T21:36:32Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-19T21:36:32Z", actor: "human:timo", revision: 7}
---
## Outcome

A write's postflight check can require that a member of the answer is present; Jira `issue.create`
requires `/id` and `/key`, and `issue.comment.add` requires `/id`, so a 2xx that names neither is
`unknown`, not `applied`.

## Why later

The second review of release 0.54.0 (2026-10-19, lane s9) proved that `issue.create` (postflight
checks `[]`) and `issue.comment.add` (checks `/body/type` only) report `applied` on a 201 with an empty
body, on `{}` and on a 204 (scratch test `verify_s9_1.rs:302` and `:350`, log `s9-verify-1.log`). The
documented rule allows it ("applied on a 2xx whose postflight holds",
`docs/local-catalog-provider.md:1256-1257`) and no documented Jira answer produces it, so 0.54.0 ships
as is. A check today compares only to an input, a literal or a boolean
(`adapters/catalog/src/lib.rs:46-50`, `adapters/catalog/spec/ess/domains/guard.yaml:35-46`), and the
new issue's id is not known before the write.

## Acceptance

- `guard.yaml` and `lib.rs` carry a `present` expectation; a postflight check with it holds only when
  the member exists and is not null.
- `adapters/catalog/providers/jira/operations.json` checks `/id` and `/key` after `issue.create` and
  `/id` after `issue.comment.add`, mirrored in the Connectors fixture `jira.yaml` and in the
  connectors-definitions repository.
- The scratch cases (empty 201, `{}`, 204) are tests and answer `unknown`.

## Scope delivered

Delivered by wave 20261019-post0540 (`git diff --name-only 52769afb3...impl/write-postflight-present-check`); every path is **cited** by that diff. The `## Scope` section above is the plan; differences are listed under *Corrections*.

- `adapters/catalog/providers/jira/operations.json`
- `adapters/catalog/spec/ess/domains/guard.yaml`
- `adapters/catalog/src/lib.rs`
- `adapters/catalog/tests/connector_declarations.rs`
- `adapters/catalog/tests/gateway_prefix.rs`
- `adapters/catalog/tests/jira_issue_writes.rs`
- `adapters/catalog/tests/jira_write_postflight_present.rs`
- `adapters/catalog/tests/write_postflight_present_adversary.rs`
- `adapters/catalog/tests/write_postflight_present_adversary_2.rs`
- `contracts/catalog/v1alpha1/semantics.md`
- `crates/connectors-catalog/src/definition.rs`
- `crates/connectors-catalog/src/import.rs`
- `crates/connectors-catalog/tests/definition_present_adversary.rs`
- `crates/connectors-catalog/tests/fixtures/definitions/connectors/jira.yaml`
- `docs/catalog-jira.md`
- `docs/local-catalog-provider.md`
- `website/docs/guides/author-a-definition.md`
- `website/docs/reference/contracts/catalog.md`

**Corrections:** the plan named 3 files; the definition format (`crates/connectors-catalog`), its Jira fixture and authoring guide, `docs/catalog-jira.md`, the catalog contract and the configuration-revision pins in 3 tests were touched too. The `connectors-definitions` repository mirror of the Jira checks is not done.

## Remaining after wave 20261019-post0540

Delivered and merged in `wave/20261019-post0540` (U2, merge 01eb808d9): the `present` expectation,
the Jira `issue.create` and `issue.comment.add` checks, the in-tree definition fixture and the `"?"`
definition syntax, with two adversary passes corrected. Gate on f96b0c013: 2858 passed, 0 failed.

Not done, so the story stays active: acceptance line 2's mirror in the connectors-definitions
repository. Its `connectors/jira.yaml` (origin/night/20261019, 1ed41fb) has no `after` on
`issue.create` (review-result:wave-20261019-post0540-u2-adversary-1, finding 2). The mirror also
needs a connectors release that understands `"?"`; an older lowerer reads `"?"` as an input
reference (adversary code reading, not run).
