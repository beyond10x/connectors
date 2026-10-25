---
format: aep.planning-md/3
id: story:owner-rotating-refresh
kind: story
status: active
title: The owner refreshes a rotating OAuth2 grant once per token and publishes a new generation
relations:
- decomposes: epic:subscription-oauth2-access
- serves: vision:independent-contract-adapters
- depends_on: story:oauth2-grant-core
- depends_on: story:oauth2-refresh-metadata-authority
- informed_by: architecture-decision-record:oauth2-grants-and-access-token-issue
- depends_on: story:connect-runs-an-oauth2-grant
scope:
- confidence: cited
  path: contracts/auth/acquisition/v1alpha1/semantics.md
- confidence: inferred
  path: crates/connectors-host/Cargo.toml
- confidence: inferred
  path: crates/connectors-host/src/local/owner
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-25T21:11:55Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"review_outcome":4}}}
- {from: "proposed", to: "active", at: "2026-10-25T21:11:55Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"review_outcome":4}}}
---
## Outcome

The owner refreshes every `oauth2_grant` connection through the `connectors.refresh` ledger, whatever its
declaration's `rotation`, and publishes the refreshed material as a new custody version and a new
`CredentialGeneration` on the same connection. One refresh token is exchanged at most once, also across
concurrent callers and an owner restart. A `rotating` declaration stores the returned refresh token; a
`non_rotating` one keeps the stored refresh token and refuses a response that carries a different one as
`uncertain`. The connection the refresh starts from is the one `story:connect-runs-an-oauth2-grant`
stores (profile, `oauth2` declaration, entry shape), so this story depends on it.

## Model

`connectors.refresh.RefreshAttempt` (`ess/domains/refresh.yaml`), owned by the metadata authority since
`story:oauth2-refresh-metadata-authority`; contract `contracts/auth/acquisition/v1alpha1/semantics.md`
§4.1 (lines 94-121) and its fake-provider cases (line 203: "Fake provider rotates then drops the response
→ `uncertain`; connection `reauthorization_required`; no second refresh call").

## Design

- Sequence per connection, serialized by the ledger, not a process mutex: `ReserveRefresh` →
  `AuthorizeExchange` (durable before the request leaves) → one refresh request through
  `connectors-oauth2` → `StoreResponse` (custody `write_new` of the complete response as a candidate
  version, `RecordCredentialGeneration`) → `PublishRefresh` (active generation and custody version
  replaced under the connection's publication fence, old version retired).
- Recovery at owner start: `Authorized` with no stored response → `QuarantineRefresh` (`uncertain`), the
  connection `Stale` with `next_action: repair_connection`; `ResponseStored` → `RecoverStoredResponse`
  then `PublishRefresh`, no request sent.
- Outcomes the caller sees are §4.1's: `refreshed`, `reauthorization_required` (`invalid_grant`; the
  connection becomes `Stale`), `insufficient_scope`, `custody_unavailable`, `uncertain`; a 429 is
  `rate_limited` and changes no state.
- A rotated response's scopes may not widen the connection's granted scopes; a missing `scope` carries the
  previous set forward (predecessor `crates/subscription-custody/src/lib.rs:627-637` on
  `compat/claude-oauth-0-7-source`).
- Contract: §4.1 of `contracts/auth/acquisition/v1alpha1/semantics.md` gains one sentence naming the
  binding: "For `oauth2_grant` the owner runs this sequence through the `connectors.refresh` ledger."
  (decision `architecture-decision-record:oauth2-grants-and-access-token-issue` item 2). The flow-table row
  for `oauth2_grant` is `story:connect-runs-an-oauth2-grant`'s; the issue-time trigger text is
  `story:connections-token-issues-an-access-token`'s.
- Entry point for the tests: an owner-internal `refresh_connection(connection)` request; the caller-facing
  trigger is the token story's.

## Acceptance

Owner tests in `crates/connectors-host` against a fake token server on loopback (rotating unless named):

- `rotating_refresh_publishes_a_new_generation`: after one refresh the connection's active generation and
  custody version differ from before, the old version is retired, the fake server saw one request.
- `concurrent_callers_share_one_exchange`: 8 concurrent refresh triggers produce one request and one
  published generation.
- `lost_response_is_uncertain_and_never_resent`: the server rotates and the owner is killed before
  `StoreResponse`; after restart the attempt is `Uncertain`, the connection `Stale`, and the server saw
  exactly one request.
- `stored_response_is_published_after_restart`: killed after `StoreResponse`; after restart the candidate
  is published and no request is sent.
- `invalid_grant_marks_the_connection_stale`, `rate_limited_changes_nothing`,
  `refresh_cannot_widen_scopes`, `refresh_without_scope_keeps_granted_scopes`.
- `non_rotating_refresh_keeps_the_stored_refresh_token`,
  `non_rotating_response_with_a_new_refresh_token_is_uncertain`.
- The ledger's commands are the metadata authority's: each transition above is recorded as the
  `connectors.refresh` command of the same name (asserted on the recorded events).
- `rg -n 'through the .connectors.refresh. ledger' contracts/auth/acquisition/v1alpha1/semantics.md`
  prints the §4.1 sentence above.

## Out of scope

Who triggers a refresh (issue-time expiry is `story:connections-token-issues-an-access-token`); the
catalog child's non-rotating refresh, which stays as it is.
