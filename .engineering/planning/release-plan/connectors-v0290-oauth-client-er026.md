---
format: aep.planning-md/3
id: release-plan:connectors-v0290-oauth-client-er026
kind: release-plan
status: active
title: 'Release 0.29.0: OAuth client credentials and Entity Runtime 0.26.0'
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "active", at: "2026-10-05T23:17:00Z", actor: "human:timo", revision: 3}
---
## Outcome and authorization

Prepare minor release 0.29.0 from origin/main 9806140. Operator rule "ready means ship"
(2026-09-25) and the daily cadence (2026-10-01): two fixes merged with green gates since
v0.28.0 (2026-10-05). The tag namespace ends at v0.28.0, an ancestor of that base. Recheck
both before tagging.

Minor, not patch: a new catalog authentication scheme, `oauth2_client_credentials`, is a
new configuration value a caller can select. Existing configurations, stored metadata,
connections and adapters are unchanged; configuration revisions of the token, basic and
oauth2_refresh schemes are unchanged.

## Released scope (v0.28.0..9806140)

| PR | Change |
|---|---|
| #95 | Catalog scheme `oauth2_client_credentials`: `{client_id, client_secret}` exchanged with the client-credentials grant and `requested_scopes` at `token_url`, bearer access token, cached and requested again on expiry, no refresh token; `docs/catalog-zendesk.md` declares `zendesk.oauth` with it. `story:catalog-client-credentials-profile`. |
| #96 | Entity Runtime 0.26.0 (from 0.25.1) and the recorded store opened with `CapturePolicy::ProviderTracked`: per read invoke 302 ms / 1.3 s / 2.3 s at 55 / 601 / 1,203 events (was 578 ms / 12.4 s / 40.2 s). `story:metadata-invoke-cost-flat-in-store-size`. |

## Runtime evidence

- 2026-10-05, a live Zendesk Support account: `zendesk.oauth` connected (identity read
  `users/me`), `tickets.incremental`, `users.incremental`, `organizations.incremental` and
  `user.show` answered 200, and a three-ticket triage (beyond10x/examples `zendesk-triage`)
  read `ticket.show`, `ticket.comments`, `user.show` and `organization.show`.
- 2026-10-06, the operator's default store (1,022+ events) with a CLI built from 9806140:
  `connections revalidate` answered `ready` in 4.2 s (zendesk) and 4.4 s (tavily), where
  0.27.0 and 0.28.0 answered `outcome_unknown` after 30 s; the same triage completed in 58 s.

## Not in this release

- Zendesk writes; per-resource Zendesk scopes (only `read` was run).
- `er.start` (complete verification per open) and `er.read_history` still grow with the
  store; `story:metadata-invoke-cost-flat-in-store-size` stays active for them.
- An adapter whose executable is re-pinned still needs `connections repair` with its
  credential (`docs/local-tavily.md`), as before.
