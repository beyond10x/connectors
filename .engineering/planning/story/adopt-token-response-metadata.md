---
format: aep.planning-md/1
id: story:adopt-token-response-metadata
kind: story
status: draft
title: An OAuth token response can carry declared metadata into the connection, not the credential store
refs:
- provider: legacy
  reference: S-004
relations:
- derived_from: epic:catalog-adoptions
scope:
- confidence: cited
  path: crates/connector-spec
- confidence: cited
  path: crates/integration-slack
- confidence: cited
  path: crates/protocol
- confidence: cited
  path: ess/system/domains/connection.yaml
- confidence: inferred
  path: providers/slack.toml
revision: 7
---
## Acceptance

Verbatim from `docs/stories/S-004-adopt-token-response-metadata.md:23`. **read**

- [ ] The OAuth2 declaration can name extra token-response fields to extract, addressed by path, into
      connection metadata. The set is **closed and declared** — named fields only, never
      "keep whatever else came back", which would make an arbitrary vendor payload a stored value
      nobody reviewed.
- [ ] Extracted values land in connection metadata and are **unrepresentable** in the credential
      store — by type if possible, by a refusal test if not. A declaration naming a field the vendor
      returns as credential material (a refresh token, a client secret) is refused at build, by name.
- [ ] Slack is the fixture: a recorded token response yields exactly `incoming_webhook.url` and
      `bot_user_id` as connection metadata, and the credential store receives nothing beyond the
      token set it already receives.
- [ ] A declared field absent from a real token response is **absent** from metadata — not an empty
      string, not a null placeholder — and its absence does not fail the acquisition. Vendors omit
      optional fields routinely; an acquisition that fails on one is worse than the gap.
- [ ] Metadata is readable by the operator and by a grant-admitted client through the connection
      surface, and is value-free in audit records (an audit row may say metadata was written; it may
      never carry the value).
- [ ] Failing-first test named — today there is no field to declare and nowhere for the value to go.

## Context

Let a provider declare the extra fields its token response returns beside the token — Slack's
`incoming_webhook.url` and `bot_user_id` are the canonical case — so those values reach **connection
metadata** at acquisition time instead of being dropped on the floor. Adopted from Nango's
`token_response_metadata` (24/957 providers), with our own invariant attached: metadata is not
credential material and never enters the credential store.

Source frontmatter: pillar Catalog · areas [catalog, connector-spec, service]. **read**

Source `note:` field, quoted: “research/catalog-precedents.md gap table: token_response_metadata, 24 of Nango's 957 providers. 'Adopt. No equivalent; today such values would be lost at acquisition time.' The domain model already fixes where they land — connection metadata, never the credential store”

## Status

`backlog` in the source. Quoted from `docs/stories/S-004-adopt-token-response-metadata.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-004-adopt-token-response-metadata.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 1 revision(s)
- Legacy id `S-004`, recorded as the reference `legacy:S-004`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

## Scope

Derived 2026-09-07 by `story-scoper`, read-only against `4d0cd30872533da40f209274f936eaeae9bf01d7` — cited.

- **Primary surface:** `crates/connector-spec` — cited; `OAuth2Spec` in `src/auth.rs:382` owns OAuth declarations; add closed extraction declarations, credential-field refusal, schema coverage and tests.
- **Acquisition and persistence:** `crates/integration-slack` — cited; `src/backend/api_runtime.rs:127` parses the token response, `src/backend.rs:262` defines stored connections, and `src/backend/connection_runtime.rs:4` constructs their readable description.
- **Connection contract:** `crates/protocol` — cited; `src/connection.rs:204` defines `ConnectionDescription`, the existing response surface requiring declared metadata.
- **Provider fixture:** `providers/slack.toml` — inferred; declare the selected response paths and exercise acquisition with a recorded response.
- **Domain modeling:** `ess/system/domains/connection.yaml:222` — cited; Connection already exists, but its fields omit token-response metadata. Model the added value type and ownership before implementation.
- **Also likely:** shared acquisition, admission/audit checks, contract projections and generated catalog artifacts — inferred; exact propagation remains unclosed.
- **Documents:** no standalone documentation deliverable — cited; acceptance requires working declaration, acquisition, custody separation and admitted reads.
- **Confidence:** medium — inferred; core owners are located, but the metadata contract and propagation boundaries need design.
- **Would collide with:** OAuth declaration/schema changes, Slack acquisition and connection persistence, Connection protocol evolution, and connection ESS modeling — inferred.
