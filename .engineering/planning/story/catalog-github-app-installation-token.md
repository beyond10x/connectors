---
format: aep.planning-md/3
id: story:catalog-github-app-installation-token
kind: story
status: implemented
title: A GitHub App installation-token scheme under mounted custody
summary: 'github_app: a 9-minute RS256 JWT exchanged for an installation token kept in memory, never returned'
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T03:14:34Z", actor: "human:timo", revision: 2, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
- {from: "proposed", to: "active", at: "2026-10-30T03:14:35Z", actor: "human:timo", revision: 3, executor: "agent:conn-d243-c1", correlation: "wave-d243-c1"}
- {from: "active", to: "implemented", at: "2026-10-30T10:11:27Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":1}}, executor: "agent:conn-d265-c4", correlation: "wave-d265-c4"}
---
## Outcome

A catalog instance authenticates as a GitHub App installation. The stored entry holds the App id,
the installation id and the App's RS256 private key. The provider signs a JWT that lives 9
minutes, exchanges it at `POST /app/installations/{installation_id}/access_tokens`
(`apps/create-installation-access-token`), sends the installation token as
`Authorization: Bearer`, and keeps it in memory until 5 minutes before its `expires_at`. The
installation token is never returned, logged, stored or written to any result or error.

It runs under `mounted_secret` custody: unlike `oauth2_refresh`, nothing rotates in the stored
entry, so the read-only mounted file is enough.

Wave D243-C1, unit C1. Design section 3, auth profile `github.app`; connectors-definitions D1
declares it as `scheme: github_app`.

## Scope

- `crates/connectors-catalog/src/definition.rs`: `AuthScheme::GitHubApp` (`github_app`) at the
  enum (about lines 130-141) and the scheme word mapping (about line 1986).
- `crates/connectors-catalog/src/import.rs` (about line 491): the same scheme on import.
- `adapters/catalog/src/local.rs` (about lines 187-209): `Scheme::GitHubApp`, its entry fields,
  the JWT, the exchange, the in-memory token cache keyed by a digest of the entry.
- The mounted custody admission of the scheme, the auth contract text and the ESS model of the
  scheme (`ess/`), and `CHANGELOG.md`.

## Acceptance (named tests)

Against a loopback HTTPS or HTTP test server standing in for `api.github.com`:
- `a_github_app_jwt_is_rs256_with_the_app_id_as_issuer_and_lives_nine_minutes`: verified with
  the public key; `iat` backdated by at most 60 s, `exp - iat` at most 600 s.
- `the_installation_token_is_sent_as_bearer_to_the_operation`
- `the_installation_token_is_reused_until_five_minutes_before_expiry`: two operations, one
  exchange.
- `a_token_within_five_minutes_of_expiry_is_exchanged_again`
- `a_refused_exchange_is_refused_by_name_with_no_operation_request`
- `the_installation_token_never_appears_in_a_result_an_error_or_a_log`
- `a_github_app_entry_missing_a_field_or_with_a_bad_key_is_refused_with_no_request`
- `mounted_secret_custody_admits_a_github_app_profile`
- `a_definition_with_scheme_github_app_lowers` (`cargo test -p connectors-catalog`)
