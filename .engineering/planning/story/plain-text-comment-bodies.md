---
format: aep.planning-md/3
id: story:plain-text-comment-bodies
kind: story
status: draft
title: A Jira comment body may be plain text wrapped by the selection
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
- informed_by: story:parity-jira-issue-create-and-comment
revision: 1
---
## Outcome

A Jira comment or description can be given as plain text: the selection wraps it in a fixed
Atlassian Document Format document with one text node. Needs the closed-body engine
(`adapters/catalog/src/closed.rs`) to admit RFC 6901 array-index segments in `body_keys` pointers
and to create arrays where a segment is an index, so `body_fixed` scalars at `/body/type`,
`/body/version`, `/body/content/0/type`, `/body/content/0/content/0/type` can sit beside the
caller leaf `/body/content/0/content/0/text`.

## Why later

On 2026-10-19 the Jira writes shipped with ADF documents sent by the caller as given (decision D2
of the parity plan); the engine builds objects only (`closed.rs` `path`, `at`, `at_mut`, `shape`,
`object`) and `body_fixed` takes scalars.

## Acceptance

- `nested_closed_body.rs` cases: an index segment creates an array element; a non-index segment
  under an array is refused by name; existing object-only selections load unchanged.
- `issue.comment.add` accepts `text` and sends the fixed document; the definition format's `body`
  notation needs no new member.
