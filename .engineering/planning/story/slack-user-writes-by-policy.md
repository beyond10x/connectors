---
format: aep.planning-md/3
id: story:slack-user-writes-by-policy
kind: story
status: draft
title: The Slack user profile carries writes; a read-only user instance is policy
relations:
- decomposes: epic:slack-complete-web-api
- depends_on: story:slack-four-profile-definition-generated
revision: 1
---
## Outcome

The user profile `slack.user` carries the writes a user token may call, and "no writes on a user
token" is per-instance policy: an instance whose allow-list names no write refuses every write at
admission, and an instance that admits one still needs an approval proof for each call.

## Change

- Replace the assertion "no write included" in
  `adapters/catalog/tests/adversary_credential_pass1.rs:290-296` with a policy test: a user
  instance whose `[adapters.<x>.permissions] operations` lists only reads refuses
  `chat.postMessage` before any request.
- The user selection gains the writes `story:slack-four-profile-definition-generated` gives it.
- Docs: the Slack guide shows a read-only user instance as configuration, not as a catalog fact.

## Acceptance

- The policy test above passes; a user instance that admits `chat.postMessage` posts only with
  an approval proof (recorded fixture).
- No engine code gains a rule naming `slack.user`.

## Scope

connectors: `adapters/catalog/tests/`, `adapters/catalog/providers/slack/user-operations.json`
(generated), the Slack guide. Inferred.
