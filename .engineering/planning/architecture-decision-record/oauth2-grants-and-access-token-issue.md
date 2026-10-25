---
format: aep.planning-md/3
id: architecture-decision-record:oauth2-grants-and-access-token-issue
kind: architecture-decision-record
status: accepted
title: OAuth2 grants keep their tokens in custody, the owner refreshes rotating grants and issues access tokens to local callers
relations:
- decides: epic:subscription-oauth2-access
revision: 4
transitions:
- {from: "proposed", to: "accepted", at: "2026-10-25T21:11:17Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
---
## Context

Timo, 2026-10-25 (verbatim, relayed by the directing session): "it must be possible in connectors lib, to
do a regular oauth2 thing, for anthropic claude code AND for codex/chatgpt. this way a token and refresh
token would be stored in connectors, and by using connectors lib/sdk or CLI we would be able to get an
always up-to-date (refreshed) token to be used".

Three existing rules stand in the way, each written for a narrower case:

- `architecture-decision-record:oauth-material-as-static-entry` rule 3 refuses a rotated refresh token,
  because "the registry has no path to publish rotated material"; it names the coordinator flow and
  `RefreshAttempt` as the target for rotating providers.
- `contracts/auth/acquisition/v1alpha1/semantics.md:75` (the OAuth-acquired non-rotating variant): "no
  persisted access token; no rotated material".
- `semantics.md` §4.1: refresh "is never caller-invocable"; the host coordinator and its metadata binding
  own the durable refresh ledger.

Both subscription providers rotate or may rotate: Codex overwrites the stored refresh token with any
`refresh_token` a refresh returns (`openai/codex@970111f6` `codex-rs/login/src/auth/manager.rs#L1647-L1690`),
and Claude Code keeps the old one only when the response omits it (Claude Code 2.1.310,
`{access_token:K,refresh_token:j=e,expires_in:Z}`).

## Decision (proposed)

1. A new profile scheme, `oauth2_grant`, carries a `connectors.oauth2.ServerDeclaration`. Its connection's
   custody entry is the validated token response: client id, access token, refresh token, access-token
   expiry and granted scopes. The access token is persisted with the refresh token, because a rotating
   refresh response must be stored as one unit and a restart must not cost a refresh.
2. The owner, not the adapter child, refreshes an `oauth2_grant` connection, through the specified
   `connectors.refresh` ledger: reserve, authorize (durable before the request is sent), store the
   response as a new custody version and `CredentialGeneration`, publish. A lost response is `uncertain`
   and requires reauthorization; there is never a second exchange of one refresh token.
3. The owner issues an access token to a local caller (`connections token`, a Rust library entry, and
   `connections launch`) and records each issue as a `connectors.oauth2.AccessTokenIssue`. Refresh stays
   maintenance: an issue triggers it only when the published token expires within the declaration's
   `refresh_lead_seconds`, which is the "expiry near" trigger §4.1 already admits. A caller cannot force a
   refresh.
4. No verb writes a token to standard output or standard error unless the caller passes `--reveal`; the
   refresh token never leaves custody by any verb, launch included.
5. `oauth2_grant` does not change the Google `static_entry` variant: its rules and rule 3 stay for
   non-rotating profiles.

## Consequences

- `contracts/auth/acquisition/v1alpha1/semantics.md` changes in three places, one owner each: the
  `oauth2_grant` flow-table row (`story:connect-runs-an-oauth2-grant`), the §4.1 sentence naming the
  ledger binding (`story:owner-rotating-refresh`), and the "Access token issue" section with §4.1's
  trigger text naming issue-time expiry (`story:connections-token-issues-an-access-token`).
- O1 (governed reach) is narrowed for these connections: a caller holding an issued token can call the
  provider outside an admitted operation. The record names every issue, its delivery and its generation.
- Accepted 2026-10-25 under the operator's standing word, below.

## Acceptance

Accepted 2026-10-25 on two statements of the operator, Timo, both relayed verbatim with their date by the
directing session `b10x-chef-orchestrator`:

- 2026-10-25, the request: "it must be possible in connectors lib, to do a regular oauth2 thing, for
  anthropic claude code AND for codex/chatgpt. this way a token and refresh token would be stored in
  connectors, and by using connectors lib/sdk or CLI we would be able to get an always up-to-date
  (refreshed) token to be used".
- 2026-10-25, on being asked again: "We made the decission long ago already, I am sick of having to
  re-decide over and over again".
