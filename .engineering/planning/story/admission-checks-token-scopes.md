---
format: aep.planning-md/3
id: story:admission-checks-token-scopes
kind: story
status: draft
title: Admission refuses an operation whose scopes the token lacks
relations:
- decomposes: epic:slack-complete-web-api
- depends_on: story:slack-method-token-scope-table
- depends_on: story:slack-four-profile-definition-generated
scope:
- confidence: inferred
  path: adapters/catalog/src
- confidence: inferred
  path: apps/connectors/src/local
- confidence: inferred
  path: ess
revision: 3
---
## Outcome

Admission refuses an operation whose required scopes the connection's token does not hold,
before any request to the provider, with a named code and the missing scopes. The token's
scopes are recorded at connect and on revalidate.

## Change

- Model first (guardrail 7): the token's granted scopes are a new fact about a connection; the
  ESS model under `ess/` declares nothing for it. Draft it there, with its relation to the
  connection `UNMAPPED:` until settled, and validate with `ess specify validate`.
- Slack: the scopes are read from the `x-oauth-scopes` response header of the identity read
  (`auth.test`). Inferred from Slack's documentation; the first acceptance bullet confirms it
  against a recorded answer before anything relies on it.
- Admission reads the operation's required scopes from the lowered operation record, the field
  `story:slack-four-profile-definition-generated` adds.
- A token whose scopes are unknown is not refused by this check; the provider's own
  `missing_scope` answer stays the refusal.

## Acceptance

- A recorded `auth.test` answer of a user token carries `x-oauth-scopes`; the recording is
  committed with its source.
- A connection whose stored scopes are empty before `connect`, and before `revalidate`, holds the
  recorded header's scopes after each.
- A token without `channels:history` is refused for `conversations.history` at admission,
  naming the missing scope; with it, the call is made.
- The refusal makes no provider request (a fixture counts requests).

## Scope

connectors: `ess/` (the model), `apps/connectors/src/local/` (admission and connect), the catalog
provider's connect in `adapters/catalog/src/`. Inferred.
