---
format: aep.planning-md/3
id: story:catalog-client-credentials-profile
kind: story
status: proposed
title: The catalog provider authenticates with OAuth client credentials (no refresh token)
relations:
- decomposes: epic:catalog-knowledge-sources
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: adapters/catalog/src/local.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime/oauth2_client_credentials.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime/oauth2_refresh.rs
- confidence: cited
  path: docs/catalog-zendesk.md
- confidence: cited
  path: docs/local-catalog-provider.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-05T13:33:01Z", actor: "human:timo", revision: 5}
---
## Problem

Zendesk retires API tokens: accounts created on or after 2026-07-28 cannot use
them, existing accounts cannot create new ones after 2026-10-27, and all stop
working on 2027-04-30 (Zendesk, "Migrating from API tokens to OAuth access
tokens", read 2026-10-05). Its refresh tokens are single-use and rotate, which
`oauth2_refresh` refuses. The catalog provider's only Zendesk profile,
`zendesk.basic`, therefore has no future, and the operator wants a credential
that never needs refreshing.

## Outcome

A catalog profile with `"scheme": "oauth2_client_credentials"` stores
`{client_id, client_secret}` once. The provider posts
`grant_type=client_credentials`, `client_id`, `client_secret` and `scope`
(the configured `requested_scopes`, space-joined) form-encoded to `token_url`
with no credential header, sends the access token as `Authorization: Bearer`,
caches it as `oauth2_refresh` does and requests a new one when it expires.
An answer carrying a `refresh_token` is a protocol failure (RFC 6749 4.4.3);
one without `expires_in` is cached for at most 24 hours; `unauthorized_client`
is an invalid credential.
The profile requires `token_url` and `requested_scopes`, refuses
`authorize_url`, `account_label`, a non-bearer or non-`Authorization` header and
an `id_token` identity, and has no acquisition: the host prompts for
`client_id` and `client_secret`.

## Acceptance

- `client_credentials_bootstrap_asks_for_the_client_only`
- `client_credentials_configuration_refusals`
- `client_credentials_grant_sends_form_and_uses_bearer`
- `client_credentials_refusals_by_code`
- `client_credentials_answer_without_expires_in_is_usable`
- `client_credentials_answer_with_null_expires_in_is_usable`

All in `adapters/catalog/tests/local_runtime/oauth2_client_credentials.rs`; the
existing `oauth2_refresh` tests stay green and existing configuration
revisions are unchanged (`oauth_existing_configuration_revisions_unchanged`).

## Not in scope

Running against a live Zendesk account; per-resource Zendesk scopes; Zendesk
writes.
