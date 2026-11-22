---
format: aep.planning-md/3
id: story:slack-user-writes-by-policy
kind: story
status: draft
title: The Slack user profile carries writes; a read-only user instance is policy
relations:
- decomposes: epic:slack-complete-web-api
- depends_on: story:slack-four-profile-definition-generated
scope:
- confidence: inferred
  path: adapters/catalog/tests/slack_user_policy.rs
- confidence: inferred
  path: website/docs/guides
revision: 3
---
## Outcome

"No writes on a user token" is per-instance policy, shown and tested as policy: an instance
whose allow-list names no write refuses every write at admission, and an instance that admits a
write still needs an approval proof for each call. The writes themselves arrive with
`story:slack-four-profile-definition-generated`, which also replaces the old catalog assertion.

## Change

- A policy test: a `slack.user` instance whose `[adapters.<x>.permissions] operations` lists only
  reads refuses `chat.postMessage` at admission, before any request.
- An approval test: a `slack.user` instance that admits `chat.postMessage` posts only with an
  approval proof (recorded fixture).
- Docs: the Slack guide shows a read-only user instance as configuration, not as a catalog fact.

## Acceptance

- The policy test passes.
- The approval test passes.
- No engine code gains a rule naming `slack.user` (git grep over `*.rs` outside tests).

## Scope

connectors: `adapters/catalog/tests/` (a new policy test file), the Slack guide under `website/`.
Inferred.
