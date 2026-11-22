---
format: aep.planning-md/3
id: story:admission-checks-token-scopes
kind: story
status: draft
title: Admission refuses an operation whose scopes the token lacks
relations:
- decomposes: epic:slack-complete-web-api
- depends_on: story:slack-method-token-scope-table
revision: 1
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
  (inferred from Slack's documentation; verify against a recorded answer before relying on it).
- Operations carry their required scopes per profile from the catalog
  (`story:slack-method-token-scope-table`).
- A token whose scopes are unknown is not refused by this check; the provider's own
  `missing_scope` answer stays the refusal.

## Acceptance

- Against a recorded answer: a token without `channels:history` is refused for
  `conversations.history` at admission, naming the missing scope; with it, the call is made.
- The refusal makes no provider request (a fixture counts requests).

## Scope

connectors: `ess/` (the model), `apps/connectors/src/local/` (admission), the catalog
provider's connect. Inferred.
