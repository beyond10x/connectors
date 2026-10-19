---
format: aep.planning-md/3
id: story:write-postflight-present-check
kind: story
status: draft
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
revision: 5
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
