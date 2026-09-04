---
format: aep.planning-md/1
id: story:the-anthropic-api-key-arrives-through-a-connect-session
kind: story
status: draft
title: The Anthropic API key arrives through a Connect Session
refs:
- provider: legacy
  reference: S-072
relations:
- derived_from: epic:subscription-custody
revision: 1
---
## Acceptance

Verbatim from `docs/stories/S-072-the-anthropic-api-key-arrives-through-a-connect-session.md:20`. **read**

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

## Context

Let a person supply their Anthropic API key through the same short-lived, single-purpose session
every other connector uses, rather than by writing it into a file by hand.

Source frontmatter: pillar Catalog · areas [providers, service] · design `../design/16-subscription-credential-custody.md`. **read**

Source `note:` field, quoted: “anthropic.api_key gains entry = connect_session so it can be pasted into a single-purpose session instead of a hand-edited file. verify = anthropic-models-list already exists and becomes the validity ping.”

## Status

`backlog` in the source. Quoted from `docs/stories/S-072-the-anthropic-api-key-arrives-through-a-connect-session.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-072-the-anthropic-api-key-arrives-through-a-connect-session.md`, which is not deleted and now names this artifact.

- First written 2026-08-25 · last touched 2026-08-25 · 1 revision(s)
- Legacy id `S-072`, recorded as the reference `legacy:S-072`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill
