---
format: aep.planning-md/3
id: story:connections-token-issues-an-access-token
kind: story
status: active
title: connections token and a library call issue a current access token, printed only on request
relations:
- decomposes: epic:subscription-oauth2-access
- serves: vision:independent-contract-adapters
- depends_on: story:owner-rotating-refresh
- depends_on: story:connect-runs-an-oauth2-grant
- informed_by: architecture-decision-record:oauth2-grants-and-access-token-issue
scope:
- confidence: cited
  path: apps/connectors
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-conformance/tests/cli_surface.rs
- confidence: inferred
  path: crates/connectors-host/src/local/owner
- confidence: cited
  path: ess/domains/cli.yaml
- confidence: cited
  path: website/docs/reference/cli.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-25T21:11:55Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":2}}}
- {from: "proposed", to: "active", at: "2026-10-25T21:11:55Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":2}}}
---
## Outcome

`connectors connections token` hands the caller a currently valid access token of one `oauth2_grant`
connection, refreshing it first when it expires within the declaration's `refresh_lead_seconds`. A Rust
caller gets the same through a public library function the CLI itself uses. By default the verb prints
only metadata; the token is written to standard output only with `--reveal`.

## Model

- `connectors.oauth2.IssueAccessToken` and `AccessTokenIssue` (`ess/domains/oauth2.yaml`): every issue is
  recorded with its generation, optional refresh attempt, expiry, scopes and delivery (`metadata_only`,
  `revealed`, `library`); every refusal records nothing.
- CLI types in `ess/domains/cli.yaml`: `ConnectionTokenInput` (`connector`, `instance`, optional
  `connection`, `reveal`) and `ConnectionTokenResult` (`connection`,
  `expires_at`, `scopes`, `refreshed`, `issue`); the binding `apps/connectors/spec/cli.yaml` adds path
  `[connections, token]`.

## Design

- Owner request `issue_access_token`: under the connection's refresh serialization, read the published
  generation; if `expires_at - now > refresh_lead_seconds` issue it (`current`), else run
  `story:owner-rotating-refresh`'s sequence and issue the published result (`refreshed`). The caller
  cannot force a refresh: the decision says so and §4.1 says a refresh "is never caller-invocable", so the
  verb takes no validity input.
- Refusals map onto `connectors.cli.FailureCode`: `not_found`, `revoked`, `forbidden` (profile is not
  `oauth2_grant`), `service_failure` with `next_action: repair_connection` (reauthorization required or
  uncertain), `unavailable` (rate limited or provider down, with retry-after when known).
- `--reveal` writes the bare token and one newline to standard output and nothing else; without it the
  result carries no token in text or JSON output. Standard error never carries a token.
- Library: one public function (crate chosen so the adapter boundary gate passes, e.g. `connectors-host`
  `local::access_token`) returning a type whose `Debug` and `Display` redact the token.
- Contract: `contracts/cli/v1alpha1/semantics.md` gains the verb row `connections token`;
  `contracts/auth/acquisition/v1alpha1/semantics.md` gains the section "Access token issue" (decision
  items 3-4), and §4.1's trigger text names issue-time expiry (decision consequence on §4.1).
- `README.md`, `website/docs/reference/cli.md` (generated) and a guide section show the verb, each command
  output pasted from a run.

## Acceptance

Against a fake rotating authorization server on loopback:

- `token_without_reveal_prints_no_token`: standard output and standard error, in text and `--format json`,
  contain neither the access nor the refresh token (byte search), and the result names `expires_at` and
  `issue`.
- `token_with_reveal_prints_only_the_token`: standard output is exactly the token and a newline; standard
  error contains neither token.
- `token_within_lead_time_refreshes_once`: a token expiring inside the lead time causes one refresh
  request and `refreshed: true`; the next call issues the same token with `refreshed: false` and no
  request.
- `token_on_a_stale_connection_refuses_with_repair_connection`, `token_on_a_static_profile_is_forbidden`.
- `library_access_token_matches_the_cli`: a Rust test outside `apps/connectors` calls the library function
  and receives the token the CLI would issue; its `Debug` output contains no token.
- Each issue is one `AccessTokenIssue` with the delivery the call used (asserted on recorded events).
- `cli_surface.rs` (count of selecting commands updated from 25 to 26) and `vocabulary_text.rs` pass;
  `connectors-build cli --check` and `connectors-docs generate --check` exit 0.
- Contract text: `rg -n 'connections token' contracts/cli/v1alpha1/semantics.md` prints the verb row;
  `rg -n '^#+ Access token issue' contracts/auth/acquisition/v1alpha1/semantics.md` prints the section
  heading; `rg -n 'issue-time' contracts/auth/acquisition/v1alpha1/semantics.md` prints the §4.1 trigger.
- Where these journeys need the disposable Secret Service, they run in
  `cargo run --locked -p connectors-build -- ignored --required`, whose report names each of them as
  passed (an ignored or skipped journey does not count).

## Out of scope

`connections launch` (`story:launch-hands-an-oauth2-access-token`); exposing a refresh token by any path.
