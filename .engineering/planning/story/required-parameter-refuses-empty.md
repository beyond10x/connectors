---
format: aep.planning-md/3
id: story:required-parameter-refuses-empty
kind: story
status: active
title: A required parameter given as an empty string is refused before any request
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:slack-single-user-lookups
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: cited
  path: adapters/catalog/src/lib.rs
- confidence: cited
  path: adapters/catalog/tests/slack_adversary.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:24Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:25Z", actor: "human:timo", revision: 4, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Outcome

A read or write whose required path or query parameter is given as an empty string is refused as
`invalid_input` naming the parameter, before any request.

## Why later

The adversary pass on the Slack one-user reads of 2026-10-19 found that `users.info` with `user`
`""` and `users.lookupByEmail` with `email` `""` are sent (`user=`), because the catalog engine's
required-parameter check is presence only (`adapters/catalog/src/lib.rs:1259`). Slack answers
`ok:false` and the read is refused as `not_found`, never answered as data; the cost is one request.
The rule is shared by every provider, so it is its own story.

## Acceptance

- `adapters/catalog/tests/slack_adversary.rs` `an_empty_user_or_email_sends_nothing` runs (no
  `#[ignore]`) and passes.
- The provider package's suites stay green; a selection that needs an empty required value is
  named in the story before the rule changes, or there is none.

## Code review

The operator-ordered code review of the repository at `23b9f1bc4` (2026-11-10, no cargo command run; "verified" means traced by reading) lists this as CR-027, taken from this story: `adapters/catalog/tests/slack_adversary.rs:905`; the source line this story cites is stale.
