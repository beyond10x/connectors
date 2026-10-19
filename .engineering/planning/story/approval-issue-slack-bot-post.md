---
format: aep.planning-md/3
id: story:approval-issue-slack-bot-post
kind: story
status: draft
title: approvals issue issues the approval its prepare named for a Slack bot post
summary: 'issue #180: issue answers invalid_input with no field, so the approved write cannot run'
refs:
- provider: github
  reference: beyond10x/connectors#180
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

`connectors approvals issue --approve-subject <subject>` issues the approval its `approvals prepare`
named for a Slack bot `chat.postMessage` under a revision-2 write policy and the `roughtime-quorum/1`
clock; when `issue` refuses with `invalid_input`, the refusal names the field it rejects.

## Why now

https://github.com/beyond10x/connectors/issues/180 (opened 2026-10-17, seen on 0.53.1): `prepare`
succeeds, `issue` answers `invalid_input` with no field, so `operations invoke --approval-file` has
no proof and the approved write cannot run. The cause is not traced; `approval_issuance.rs:131-359`
holds about 15 field-less `InvalidInput` sites (post-0.54.0 re-plan, 2026-10-19). Whether 0.54.0
still shows it is not known.

## Acceptance

- From fixtures, no live credential: a Slack bot connection, a revision-2 write policy and the quorum
  clock; `approvals prepare` then `approvals issue --approve-subject <subject from prepare>` issues
  the approval, and `operations invoke --approval-file` accepts it.
- Every `invalid_input` that `approvals issue` can answer names its field (a test per site in
  `crates/connectors-host/src/local/owner/approval_issuance.rs`).
- The test that reproduces the issue's sequence fails on 83aee7c96 or the story records that it
  passes there and names the release that fixed it.
