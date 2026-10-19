---
format: aep.planning-md/3
id: story:required-body-key-refuses-null
kind: story
status: draft
title: A required body key sent as null is refused before any request
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:parity-jira-issue-create-and-comment
scope:
- confidence: cited
  path: adapters/catalog/src/closed.rs
- confidence: cited
  path: adapters/catalog/tests/adversary_catalog_body_types.rs
- confidence: cited
  path: adapters/catalog/tests/jira_issue_writes_adversary.rs
revision: 3
---
## Outcome

A write whose `body_required` key is JSON `null` is refused before any request, like a write that
omits the key; the declared input schema says the same.

## Why later

The adversary pass on the Jira writes of 2026-10-19 found that `issue.comment.add` sends
`{"type":"doc","version":1,"content":null}`: the closed-body engine reads "required" as "present"
(`adapters/catalog/src/closed.rs:366`), and `adapters/catalog/tests/adversary_catalog_body_types.rs:113`
asserts that the schema and `prepare` both admit `null` for a required untyped key. Changing the rule
changes an asserted behaviour of every provider, so it is its own story. Jira would answer `400`; the
cost today is one spent approval.

## Acceptance

- `adapters/catalog/tests/jira_issue_writes_adversary.rs`
  `a_comment_whose_content_is_null_is_refused_before_any_request` runs (no `#[ignore]`) and passes.
- The line-113 case of `adversary_catalog_body_types.rs` expects a refusal, and the input schema
  declares the required untyped key non-null, so schema and `prepare` still agree.
- A `body_any_of` member given as JSON `null` (`adapters/catalog/src/closed.rs:400`) either does not
  count as the one change, or the contract names `null` as a clear for that provider and a test pins
  it (found by the 0.54.0 review, lane r1, 2026-10-19).
