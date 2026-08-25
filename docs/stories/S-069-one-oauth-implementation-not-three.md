---
id: S-069
title: "One OAuth implementation, not three"
pillar: Platform
status: in-progress
priority: 1
design:
epic: oauth-consolidation
areas: [service, integration-gitlab, integration-jira, integration-slack]
note: "GitLab, Jira and Slack each hand-rolled authorization-code OAuth; only GitLab does PKCE. Extract one crate from the GitLab implementation and migrate the other two onto it, behaviour-preserving. Prerequisite Timo chose for the Claude connector work; also unblocks S-013's loopback callback."
---

# One OAuth implementation, not three

## Goal

Replace three hand-rolled OAuth implementations with one, so the next connector that needs a
browser flow inherits PKCE, single-use state, bounded exchange and crash-safe refresh instead of
copying whichever neighbour it read first.

## Acceptance

- [ ] A new `crates/connector-oauth` carries: state generation with a single-use TTL map, S256
      challenge derivation, authorize-URL construction, bounded token exchange with per-field
      validation, and refresh with a configurable skew under double-checked locking.
- [ ] `integration-gitlab` migrates first and is the extraction source — it is the only current
      PKCE implementation (`crates/integration-gitlab/src/backend.rs:678-687`, `:739-750`, `:752`,
      `:806-855`). Its behaviour is unchanged, proven by its existing tests passing untouched.
- [ ] `integration-jira` migrates second. Its refresh path is the best in the repo
      (`crates/integration-jira/src/backend/auth.rs:453-536` skew + double-checked lock,
      `:397-432` crash recovery) and the extracted crate must not regress it — if the shared shape
      cannot express it, the shared shape is wrong.
- [ ] `integration-slack` migrates third, keeping its egress-gate routing
      (`crates/integration-slack/src/backend/api_runtime.rs:43-179`) rather than a bare client.
- [ ] **Behaviour-preserving.** Jira and Slack stay non-PKCE; neither provider declares
      `public_client`, so adding PKCE here would be an undeclared change to a live integration.
- [ ] One commit per integration, gate green between each, so a regression bisects to one vendor.
- [ ] The prepared-transaction commit path stays the only way a token pair is persisted; no point
      write appears in the extracted crate.

## Progress

- 2026-08-25 — `crates/connector-oauth` landed (24 tests): `random_token`, `Pkce`, `authorize_url`,
  `PendingStates`, `validate`/`TokenPolicy`, `parse_scopes`/`ScopePolicy`, `refresh_due`,
  `DEFAULT_PENDING_CAPACITY`. Classified as a host library in `dependency_fence.rs` — it owns no
  transport and reads no clock.
- 2026-08-25 — GitLab migrated. Its five existing tests do not reach the OAuth path, so four
  characterisation tests were written with the move (9 total) and moved to `src/backend_tests.rs`
  behind an `include!`, following Slack's precedent, to stay inside the module-size waiver.
- 2026-08-25 — Jira migrated: three policies (exchange / refresh / `client_credentials` service),
  and both halves of the double-checked refresh now call `refresh_due`. Three characterisation
  tests added (12 total).
- 2026-08-25 — Slack migrated **partially, deliberately** — see the note below (25 tests, its own
  suite unchanged).
- Gate green: `bash scripts/gate.sh` exit 0. `clippy -D warnings` clean for all three integrations
  and the root workspace.
- Not committed; awaiting review.

## Notes

### Slack shares the state table and nothing else, on purpose

Slack was migrated onto `PendingStates` only. Its other two arms were left alone because forcing
them into the shared shape would have changed a live request or turned the helper into a
configuration soup:

- **Authorize URL.** Slack does not send `response_type` (`/oauth/v2_user/authorize` treats it as
  implicit) and orders its parameters differently. The shared `authorize_url` always sends
  `response_type=code`, which RFC 6749 requires and which GitLab and Jira send. Routing Slack
  through it would add a parameter to a request that works today.
- **Token response.** Slack's is not the RFC shape at all: `{ok, authed_user{id,scope,...},
  team{id}, access_token}`, validated on an `xoxp-` prefix, a 2048-byte bound, a team match and
  `valid_slack_id`. `TokenPolicy` describes `token_type`/`expires_in`/`scope`, none of which
  appear at Slack's top level.
- **Scope parsing.** Slack splits on both `,` and ` `, and filters per scope by charset
  (lowercase, digits, `:.-_`) and a 128-byte bound. `ScopePolicy` has a separator and a retain
  list, not a charset predicate.

Two of three is the honest result. A `TokenPolicy` stretched to cover Slack would have been a
shared type no caller could read.

### Behaviour changes, stated

- **The pending-state table is now bounded** at `DEFAULT_PENDING_CAPACITY` (1024) in all three. It
  was unbounded, and every connect session inserts one. Expired entries are swept before the bound
  is consulted, so only a genuine flood reaches it.
- **GitLab's refresh response is now length-bounded** at 4096, as its exchange response already
  was. Previously unbounded; a GitLab token is ~88 characters, so this is unreachable in practice.
- `owns_hosted_oauth_state` deliberately still ignores expiry (`contains_any`). Making it
  expiry-aware turned an expired callback from `Refused` into `NotFound`, because the dispatcher
  then found no claimant — caught during the migration and reverted.

- Measured 2026-08-25: there is no shared OAuth module, no `crates/oauth-*`, no helper. A fourth
  connector needing a browser flow means a fourth copy.
- Feeds [S-013](S-013-connect-session-oauth-custody-in-personal-posture.md): the personal posture
  has no loopback `/callback` route at all —
  `crates/connect-session-transport/src/lib.rs` binds a loopback listener serving only `GET /` and
  `POST /complete`. The extracted crate is where that route's validation belongs.
- Watch the callback constraint at `crates/server/src/hosted/connect.rs:23-29`: `code` is capped at
  1024 characters of `[A-Za-z0-9._-]`. Confirm against real vendor codes rather than assuming.
