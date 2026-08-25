---
id: S-072
title: "The Anthropic API key arrives through a Connect Session"
pillar: Catalog
status: backlog
priority:
design: ../design/16-subscription-credential-custody.md
epic: subscription-custody
areas: [providers, service]
note: "anthropic.api_key gains entry = connect_session so it can be pasted into a single-purpose session instead of a hand-edited file. verify = anthropic-models-list already exists and becomes the validity ping."
---

# The Anthropic API key arrives through a Connect Session

## Goal

Let a person supply their Anthropic API key through the same short-lived, single-purpose session
every other connector uses, rather than by writing it into a file by hand.

## Acceptance

- [ ] `anthropic.api_key` gains `entry = "connect_session"` in `providers/anthropic.toml`.
- [ ] The provider's `id`, `authority`, auth method **names**, services and operations are
      unchanged, so `catalog_invariants.rs:1226` passes untouched.
- [ ] The declared `verify = "anthropic-models-list"` is what a completed connection runs as its
      validity ping. On success the connection records a value-free fingerprint and a
      `last_verified_at`; on failure the session refuses with the upstream's reason named.
- [ ] No identity is claimed or displayed. Anthropic exposes no profile or email on any credential,
      and `/v1/organizations/me` needs an Admin key and returns organization fields only — the UI
      says "verified", never who.
- [ ] The Connect Session invariants hold: single-purpose, short-lived, never returns credential
      material to its creator, terminal event names the connection id and nothing else.

## Progress
- (not started)

## Notes

- `anthropic.admin_key` is out of scope. An Admin key can read every user and every API key in the
  organization; admitting it through a self-service session is a much larger blast radius and wants
  its own story and its own operator gate.
- The personal posture already connects this provider end to end today — the generic enrol path
  prompts hidden, stores through `integration_catalog::credential_address`, and writes the
  `[[catalog]]` entry. This story is what makes the same thing true in a deployment, together with
  [S-073](S-073-the-hosted-posture-connects-a-catalogued-provider.md).
