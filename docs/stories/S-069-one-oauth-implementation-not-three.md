---
id: S-069
title: "One OAuth implementation, not three"
pillar: Platform
status: done
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

- [x] A new `crates/connector-oauth` carries: state generation with a single-use TTL map, S256
      challenge derivation, authorize-URL construction, bounded token exchange with per-field
      validation, and refresh with a configurable skew under double-checked locking.
- [x] `integration-gitlab` migrates first and is the extraction source — it is the only current
      PKCE implementation (`crates/integration-gitlab/src/backend.rs:678-687`, `:739-750`, `:752`,
      `:806-855`). Its behaviour is unchanged, proven by its existing tests passing untouched.
- [x] `integration-jira` migrates second. Its refresh path is the best in the repo
      (`crates/integration-jira/src/backend/auth.rs:479-518` skew + double-checked lock,
      `:416-451` crash recovery) and the extracted crate must not regress it — if the shared shape
      cannot express it, the shared shape is wrong.
- [x] `integration-slack` migrates third, keeping its egress-gate routing
      (`crates/integration-slack/src/backend/api_runtime.rs:43-179`) rather than a bare client.
      **Partially, on purpose** — only the pending-state table moved; see the note below. Two of
      three is the honest result, not a shortfall hidden behind a tick.
- [x] **Behaviour-preserving.** Jira and Slack stay non-PKCE; neither provider declares
      `public_client`, so adding PKCE here would be an undeclared change to a live integration.
- [x] One commit per integration, gate green between each, so a regression bisects to one vendor.
      `f7149ab` GitLab, `ce04be1` Jira, `d9fc657` Slack. The first draft landed all three as one
      commit; independent review caught it and the branch was split before it reached `main`.
- [x] The prepared-transaction commit path stays the only way a token pair is persisted; no point
      write appears in the extracted crate.

## Behaviour changes, stated

The Acceptance says behaviour-preserving. These four are the exceptions, each deliberate and none
observed against a live vendor. Recorded here rather than left for a reader to discover:

1. **GitLab's token bound tightened.** Access and refresh secrets over 4096 bytes are now refused
   by `EXCHANGE_POLICY`/`REFRESH_POLICY`; the inline code had no such bound. A differential run of
   the old inline conditions against the new policies over 15,500 combinations found exactly these
   360 rows and no others.
2. **Jira's authorize-URL parameter order changed** — `audience, client_id, scope, redirect_uri,
   state, response_type, prompt` became `client_id, redirect_uri, response_type, scope, state,
   audience, prompt`. Key set, values and percent-encoding are byte-identical; only the order
   moved, and order is not significant in a query string. GitLab's authorize URL is byte-identical.
3. **`canonical_scopes` changed from per-element filtering to join-and-resplit.** An array element
   that itself contains whitespace — `["api sudo"]` — was dropped entirely before and now
   contributes `api`. Neither GitLab nor Atlassian returns such an element; the new reading is the
   more defensible one, and it is stated rather than assumed.
4. **The pending-state table is now bounded** at `DEFAULT_PENDING_CAPACITY` (1024) in all three.
   It was unbounded, and every connect session inserts one. Expired entries are swept before the
   bound is consulted, so only a genuine flood reaches it. See *Known gap*.
5. **`authorize_url` now clears the origin's query and fragment.** Previously the shared builder
   appended to whatever the origin carried. Unreachable from either shipped caller — GitLab
   validates its origin query- and fragment-free, Jira's is a constant — but S-013's loopback
   callback inherits this builder, so it was fixed rather than documented.

`owns_hosted_oauth_state` deliberately still ignores expiry (`contains_any`). Making it
expiry-aware turned an expired callback from `Refused` into `NotFound`, because the dispatcher then
found no claimant — caught during the migration, reverted, and now pinned by the doc comment on
`PendingStates::contains`, which previously repeated `contains_any`'s claim and would have re-sold
the same mistake to the next reader.

**Slack is migrated only partially, on purpose.** `PendingStates` replaced its state map; its
authorize URL (which omits `response_type`), its `xoxp-`-prefix token judgement with a 2048-byte
bound, and its per-scope charset parser stay as they are. Forcing them through `TokenPolicy` would
either change a live request or make the policy unreadable.

## Known gap

`DEFAULT_PENDING_CAPACITY` (1024) is a **shared denial surface**. 1024 concurrently-live pendings
within `connect_session_ttl_seconds` makes every further connect session for that integration
return `connection_unavailable`, process-wide across tenants. Entries self-expire, so it cannot
wedge permanently. Stated because the bound is declared in this story and its failure mode was not.

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

- Measured 2026-08-25: there is no shared OAuth module, no `crates/oauth-*`, no helper. A fourth
  connector needing a browser flow means a fourth copy.
- Feeds [S-013](S-013-connect-session-oauth-custody-in-personal-posture.md): the personal posture
  has no loopback `/callback` route at all —
  `crates/connect-session-transport/src/lib.rs` binds a loopback listener serving only `GET /` and
  `POST /complete`. The extracted crate is where that route's validation belongs.
- Watch the callback constraint at `crates/server/src/hosted/connect.rs:23-29`: `code` is capped at
  1024 characters of `[A-Za-z0-9._-]`. Confirm against real vendor codes rather than assuming.
