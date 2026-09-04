---
id: S-004
title: "An OAuth token response can carry declared metadata into the connection, not the credential store"
pillar: Catalog
status: backlog
priority:
design:
epic: catalog-adoptions
areas: [catalog, connector-spec, service]
note: "research/catalog-precedents.md gap table: token_response_metadata, 24 of Nango's 957 providers. 'Adopt. No equivalent; today such values would be lost at acquisition time.' The domain model already fixes where they land — connection metadata, never the credential store"
---

# An OAuth token response can carry declared metadata into the connection, not the credential store

## Goal

Let a provider declare the extra fields its token response returns beside the token — Slack's
`incoming_webhook.url` and `bot_user_id` are the canonical case — so those values reach **connection
metadata** at acquisition time instead of being dropped on the floor. Adopted from Nango's
`token_response_metadata` (24/957 providers), with our own invariant attached: metadata is not
credential material and never enters the credential store.

## Acceptance

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

## Progress
- (not started)

## Notes

- Source: [research/catalog-precedents.md](../research/catalog-precedents.md) § gap table and
  § consequences item 2 (ordered adoption list: `token_response_metadata`, header-name rate-limit
  retry, per-service verification probes) — this is the first of the three.
- Domain model, Credential: *"`token_response_metadata`-style extraction … lands in **connection
  metadata**, never in the credential store."* That invariant is the reason this is a catalog story
  with a service half, not a credential-store story.
- Sibling adoptions: [S-005](S-005-header-name-rate-limit-retry.md),
  [S-006](S-006-per-service-verification-probes.md).

## Superseded by

`story:adopt-token-response-metadata` in the AEP planning store, at
`.engineering/planning/story/adopt-token-response-metadata.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
