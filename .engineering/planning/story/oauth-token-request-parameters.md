---
format: aep.planning-md/3
id: story:oauth-token-request-parameters
kind: story
status: archived
title: An auth profile adds fixed parameters to the token request
summary: A declared list of fixed, non-secret parameters, for providers such as Launchpad
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-13T10:50:32Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-13T10:50:32Z", actor: "human:operator", revision: 3}
- {from: "active", to: "archived", at: "2026-11-13T10:52:12Z", actor: "human:operator", revision: 4}
---
# An auth profile adds fixed parameters to the token request

## Need

Launchpad, Basecamp's sign-in service, expects `type=web_server` on the code exchange and
`type=refresh` on a refresh. The host sends only the standard `grant_type`
(`crates/connectors-host/src/local/oauth.rs:595`, `adapters/catalog/src/local.rs:1241`) and refuses
a `token_url` with a query (`oauth.rs:566`), which stays refused.

## Acceptance

- An auth profile may declare, per token request (code exchange, refresh), a list of extra form or
  query parameters with fixed values from the definition.
- A value cannot be a secret, a caller input or a template; a secret-typed or templated value is
  refused when the definition loads, by a case that fails first.
- A `token_url` with a query is still refused.
- Every existing definition lowers and behaves byte-equal; the catalog format reference documents
  the field.
- An independent security review passes before merge.
