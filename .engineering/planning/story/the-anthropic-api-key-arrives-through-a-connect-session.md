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
scope:
- confidence: cited
  path: crates/integration-catalog
- confidence: inferred
  path: crates/server
- confidence: inferred
  path: crates/service
revision: 4
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

## Scope

Derived 2026-09-07 by `story-scoper`, read-only against `4d0cd30872533da40f209274f936eaeae9bf01d7` — cited.

- **Primary surface:** `crates/integration-catalog` — cited; `src/hosted.rs:628` discards verification failure details and `:910` owns completion messaging. Existing Anthropic session fixtures begin at `:1093`; scope remaining work to verification outcomes and missing acceptance coverage.
- **Completion contract:** `crates/service` — inferred; `src/runtime.rs:494` defines the closed `HostedCompletionError` vocabulary, which currently cannot carry a named verification reason.
- **HTTP boundary:** `crates/server` — inferred; `src/hosted/connect.rs:141` maps completion outcomes to responses and would carry any added bounded, value-free verification diagnosis.
- **Already present:** declared acquisition, provider-selected verification, stored fingerprint/time, expiring capability-bound sessions, and Anthropic credential-isolation fixtures — cited.
- **Documents:** no standalone documentation change required — cited.
- **Confidence:** medium — inferred; concrete residual behavior is located, but terminal-event delivery and personal-posture equivalence remain unestablished.
- **Would collide with:** generic hosted catalog completion, shared completion-error vocabulary, browser completion responses and their fixtures — inferred.
