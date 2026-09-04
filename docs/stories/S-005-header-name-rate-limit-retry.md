---
id: S-005
title: "A rate limit the vendor discloses at runtime can be declared by header name"
pillar: Catalog
status: backlog
priority:
design:
epic: catalog-adoptions
areas: [catalog, connector-spec]
note: "ported from flux-connectors C-224: quirks.rate_limit takes a fixed requests/per_seconds pair, and two shipped connectors declined to declare one for unrelated reasons (hubspot: the limit is a function of the customer's tier; discord: per-route buckets discovered from response headers). Two independent refusals is the declaration's shape being too narrow, not two lazy connectors. Nango declares retry on 160 of 957 providers"
---

# A rate limit the vendor discloses at runtime can be declared by header name

## Goal

Let a connector state a rate limit it genuinely knows the shape of, so the information reaches the
canonical document instead of surviving only in a description a machine cannot act on — and so the
the declared invocation path, and any later operator-only proxy, can normalize backoff from catalog
metadata rather than rediscovering it at runtime.

## What the predecessor measured

`quirks.rate_limit` took a fixed `requests` / `per_seconds` pair. Two shipped connectors declined it:

| connector | why it declined |
|---|---|
| `hubspot` | the limit is a function of the customer's **tier**, so no single pair is true for all operators |
| `discord`  | the limit is **per-route**, bucketed by major path parameter, and *discovered* from `X-RateLimit-*` / `Retry-After` response headers |

Discord's is the sharper case: the one published figure (a global 50 req/s per bot) is shared across
every route, so writing it per-operation would state six allowances no individual route has.
Declaring it would be **less** true than declaring nothing.

## Acceptance

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

## Progress
- (not started)

## Notes

- Predecessor: [`C-224 — rate limit cannot express a discovered budget`](https://github.com/codewandler/flux-connectors/blob/main/docs/stories/C-224-ratelimit-cannot-express-a-discovered-budget.md)
  (status `ready` there, priority 3) — read it, including its warning not to let the story grow into
  implementing backoff, and its pointer to the quirks-as-control-flow position (C-12) so this does
  not become a fourth vocabulary.
- Research grounding: [catalog-precedents.md](../research/catalog-precedents.md) — Nango declares
  `retry` on 160/957 providers with a minimal `after`/`at` header-name shape; adopt that as the first
  supported discovered form rather than inventing one.
- This is a `connector-spec`/schema change: it collides with any other story touching the same public
  surface, and should run solo or early in a wave.

## Superseded by

`story:header-name-rate-limit-retry` in the AEP planning store, at
`.engineering/planning/story/header-name-rate-limit-retry.md`, now carries this story. Migrated 2026-09-04 by the
`aep-planning:story-migration` skill; this file is kept as the source it was migrated from and is
not the place the rung is recorded any more. The artifact quotes this file's `## Acceptance`
verbatim and names it in its own `## Provenance`.
