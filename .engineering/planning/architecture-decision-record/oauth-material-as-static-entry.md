---
format: aep.planning-md/3
id: architecture-decision-record:oauth-material-as-static-entry
kind: architecture-decision-record
status: accepted
title: OAuth-acquired, non-rotating refresh material is held as a static entry
relations:
- informed_by: epic:google-workspace-reads
revision: 3
transitions:
- {from: "proposed", to: "accepted", at: "2026-09-30T13:13:59Z", actor: "human:timo", revision: 3}
---
## Context

Google APIs need OAuth 2.0 access tokens that expire; a static token in custody is usable for a
prototype only. `contracts/auth/acquisition/v1alpha1/semantics.md` specifies
`oauth2_authorization_code` as a coordinator flow (§4.0, line 75) with begin/complete ports, a
trusted UI channel and host-owned refresh (`ess/domains/refresh.yaml`, `RefreshAttempt`). None of
it is implemented. The registry admits `static_entry` only
(`crates/connectors-host/src/local/registry.rs:60-61`), and the `static_entry` row forbids "OAuth
endpoints, OAuth grants, redirect or browser PKCE fields" (`semantics.md:74`).

Facts that make a smaller design correct for Google:

- Google does not rotate refresh tokens for installed apps; the refresh request needs `client_id`,
  `grant_type=refresh_token`, `refresh_token`, with `client_secret` optional
  (developers.google.com/identity/protocols/oauth2/native-app). The stored material therefore never
  changes after acquisition.
- Custody versions are immutable and a replacement is a new version published through the registry
  (`crates/connectors-host/src/local/keyring/custody.rs:159`); with immutable material none is needed.
- The adapter child receives the secret on every request and lives across invokes
  (`crates/connectors-host/src/local/runtime/process.rs:200-236`,
  `crates/connectors-host/src/local/owner/supervisor.rs:617-640`).
- Owner and child run with a cleared environment and no TTY
  (`crates/connectors-host/src/local/owner/transport.rs:642-652`); only the CLI process can reach a
  browser.

## Decision

1. The CLI acquires the material during `connections connect` / `repair`, inside the owner's
   300 s capture window (the profile, which declares the authorize URL, token URL and scopes, is
   known only after `begin`), by loopback redirect + PKCE S256 with a 240 s flow timeout. The result `{client_id, client_secret, refresh_token}` enters the
   existing static-entry capture unchanged. Owner, registry and custody are not changed.
2. The catalog adapter child holds refresh: a profile scheme `oauth2_refresh` exchanges the refresh
   token at the configured `token_url` for an access token, caches it in process memory until
   60 s before `expires_in`, and never persists it. `credential_expires_at_ms` stays `None`.
3. A refresh response that carries a `refresh_token` different from the stored one is refused as an
   invalid credential, not stored: this profile is declared non-rotating, and the registry has no
   path to publish rotated material. Rotating providers need the coordinator flow and
   `RefreshAttempt`; this decision does not cover them.
4. `semantics.md` gains a `static_entry` variant, "OAuth-acquired non-rotating", naming these rules,
   and the `registry.rs:60-61` comment and `docs/local-connection-registry.md:121` are amended to
   match.

## Consequences

- One OAuth consent per connection; with one catalog configuration per Google API that is up to
  four consents, repeated every 7 days for an external app in Testing.
- The access-token cache dies with the child (owner idle exit, 600 s), costing one token request
  per restart.
- Jira and GitLab user OAuth fit this decision only where their refresh tokens do not rotate; Slack
  rotating tokens do not.
- The coordinator flow in `auth.acquisition/v1alpha1` stays the target for rotating providers and
  for hosted deployments; nothing here closes it.
