---
format: aep.planning-md/1
id: story:header-name-rate-limit-retry
kind: story
status: draft
title: A rate limit the vendor discloses at runtime can be declared by header name
refs:
- provider: legacy
  reference: S-005
relations:
- derived_from: epic:catalog-adoptions
scope:
- confidence: cited
  path: crates/catalog-build
- confidence: cited
  path: crates/connector-spec
- confidence: cited
  path: providers/discord.toml
- confidence: cited
  path: providers/hubspot.toml
revision: 6
---
## Acceptance

Verbatim from `docs/stories/S-005-header-name-rate-limit-retry.md:35`. **read**

- [ ] The rate-limit declaration distinguishes at least three states: a **fixed** pair (what exists),
      a **discovered** budget that names the response headers carrying it, and an
      **unknown-by-tier** budget. Whether that is one enum or a struct with an optional pair is the
      implementor's call; record the reason where the schema documents itself.
- [ ] A discovered budget names the **actual headers** (`Retry-After`, `X-RateLimit-Remaining`,
      `X-RateLimit-Reset`, …) rather than implying a convention. Vendors disagree about these, and
      the disagreement is the entire difficulty; a convention would be a guess stored as data.
- [ ] `hubspot` and `discord` both stop being exceptions: each declares what it knows, and the prose
      in its description shrinks to what the declaration cannot carry. Two spellings of one fact is
      the defect to avoid here.
- [ ] What a consumer is expected to **do** with a discovered budget is **stated** in the schema's
      own documentation and the consuming path is **named** (the invocation egress path) — a
      declaration nothing can act on is prose with a schema. Implementing the backoff is a separate
      story and is explicitly out of scope here.
- [ ] Failing-first test: a connector declares a header-discovered budget and it reaches the canonical
      document. It cannot be expressed today. Name the test.

## Context

Let a connector state a rate limit it genuinely knows the shape of, so the information reaches the
canonical document instead of surviving only in a description a machine cannot act on — and so the
the declared invocation path, and any later operator-only proxy, can normalize backoff from catalog
metadata rather than rediscovering it at runtime.

Source frontmatter: pillar Catalog · areas [catalog, connector-spec]. **read**

Source `note:` field, quoted: “ported from flux-connectors C-224: quirks.rate_limit takes a fixed requests/per_seconds pair, and two shipped connectors declined to declare one for unrelated reasons (hubspot: the limit is a function of the customer's tier; discord: per-route buckets discovered from response headers). Two independent refusals is the declaration's shape being too narrow, not two lazy connectors. Nango declares retry on 160 of 957 providers”

## Status

`backlog` in the source. Quoted from `docs/stories/S-005-header-name-rate-limit-retry.md:5`: `status: backlog`. **read**

## Provenance

Migrated from `docs/stories/S-005-header-name-rate-limit-retry.md`, which is not deleted and now names this artifact.

- First written 2026-08-13 · last touched 2026-08-13 · 3 revision(s)
- Legacy id `S-005`, recorded as the reference `legacy:S-005`
- Migrated 2026-09-04 by the `aep-planning:story-migration` skill

## Scope

Derived 2026-09-07 by `story-scoper` at released base 4d0cd308 — cited.

- **Declaration surface:** `crates/connector-spec` — cited; src/rate_limit.rs:8 defines the fixed pair, while ConditionalRateLimit permits an absent numeric allowance but carries no header mapping.
- **Canonical projection and regressions:** `crates/catalog-build` — cited; src/document.rs:340 serializes rate declarations; src/document_schema.rs:503 still requires requests/per_seconds; existing provider assertions belong in tests/main/catalog_invariants.rs.
- **Provider declaration:** `providers/hubspot.toml:107` — cited; tier-dependent limits remain deliberately undeclared.
- **Provider declaration:** `providers/discord.toml:112` — cited; per-route response-header discovery remains prose.
- **Consumer boundary:** invocation egress already normalizes HTTP 429 using literal retry-after; document how declared headers would be consumed, without implementing backoff — cited.
- **Proposed deciding regression:** header_discovered_rate_reaches_canonical_document, retaining fixed-rate compatibility and explicit unknown-by-tier behavior — inferred.
- **Confidence:** medium; missing vocabulary and owners are clear, but representation and schema compatibility remain undecided — inferred.
- **Would collide with:** rate declaration/IR validation, canonical schema generation and the two provider definitions — cited.
