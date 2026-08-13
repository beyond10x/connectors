---
id: S-018
title: "The web explorer works against the site JSON M1 actually emits"
pillar: Catalog
status: ready
priority: 7
design:
epic: post-m1
areas: [web, catalog-build]
note: "M1 report: web/ is written against `operations[].flux` and a `core` object that the projection no longer emits, and the node suites are in no gate — not in cargo, not in CI — so nothing said so. Four suites, ~2.9k lines of test, currently asserting a site that does not exist"
---

# The web explorer works against the site JSON M1 actually emits

## Goal

Make the public explorer true again: render each operation's **request template** — the thing the
runtime will actually send — in place of the emitted-Flux snippet that no longer exists, delete the
core explorer whose input was dropped, and put the node suite into a gate so the site's correctness
is checked rather than assumed.

## What M1 left

The site projection emits `{ generator, providers, schema_version: 3 }`, and an operation record
carries `method`, `path`, `parameters`, `input_schema`, `body_schema`, `response_schema`, `risk`,
`idempotency`, `direction`, `credentials`, `hosts`, `semantic_effects`, `spec_source`, `status`.
There is **no `flux` key on an operation and no `core` object anywhere**. The site still expects both:

| Where | What it expects that is gone |
|---|---|
| `web/data/catalog.mts` | `flux: Published<string>`, `flux_ast`, `core: CoreCatalog \| null`, `allCoreEntries` / `coreEntryHref` / `coreEntryById`, and an `operationSummary` that reads `operation.flux.split('\n', 1)` |
| `web/.vitepress/theme/components/FluxSource.vue` | renders "the bytes the emitter produced, unedited" — there is no emitter |
| `CoreExplorer.vue`, `CoreDetail.vue`, `web/core/[kind]/[name].md` + `.paths.mts` | the whole core-catalogue section, whose input (`specs/flux/core-v1.json`) is orphaned (S-022) |
| `web/test/*.test.mjs` | four suites — `catalog_types`, `explorer` (~2.1k lines), `ci_gate`, `release_assets` |

None of this failed during M1 because **the node tests are not in the cargo gate and were not run**.
`ci_gate.test.mjs` cannot pass here at all: it asserts that a workflow under `.github/workflows/`
runs the web suite, and this repository has no `.github/` directory (S-020).

## Acceptance

- [ ] `npm ci && npm run build && npm test` in `web/` is **green against the committed
      `web/public/catalog.json`** — not against a fixture — and that command is the documented web
      gate in AGENTS.md.
- [ ] Per-operation rendering shows **the request the runtime will send**: method, the URL template
      with its `{…}` slots intact, constant headers, the body template, and each parameter's
      position — the repository-owned replacement for the per-operation Flux snippet. The values come from
      the projection; the view derives none of them. The canonical document already carries
      `request { method, url, headers }`; the site record today carries only `method` and `path`, so
      the projection gains what the view needs rather than the view reconstructing it in JavaScript.
- [ ] `FluxSource.vue` is deleted, and no reference to `operations[].flux`, `flux_ast` or the
      Flux-derived summary survives in `web/`. `catalog.mts`'s types describe the shipped document
      exactly, asserted by `catalog_types.test.mjs` over the real committed JSON.
- [ ] The core explorer is deleted **whole**: `CoreExplorer.vue`, `CoreDetail.vue`, `web/core/**`,
      the `CoreCatalog` types and their three helpers, and every route, nav entry and link that
      reaches them. A build with VitePress's dead-link checking on proves nothing dangles.
- [ ] Each of the four node suites has a recorded outcome, and none is left asserting a fiction:
      `explorer` and `catalog_types` reworked to the new shape; `ci_gate` true once S-020 lands;
      `release_assets` — which asserts a tag workflow attaching `catalog.pack` + `.sha256` — either
      deleted or explicitly parked against the milestone that creates a release train, since
      architecture §8 says there are no release artifacts pre-v1.
- [ ] The node suite runs in CI (S-020's web job) and lands green there; a stale site can no longer
      be green by not being run.
- [ ] **Failing-first, and recorded:** run the suite before touching a view, and put the actual
      failure list in this story's Progress. It is the measurement the M1 report inferred but did not
      take.

## Progress
- (not started)

## Notes

- Do not repair the types to make the old views compile. The site's rule is that it **never
  hand-maintains catalogue data** (`crates/catalog-build/src/site.rs` header: "that is the
  action-proxy failure this repository exists to correct, re-enacted in JavaScript"). Anything the
  view needs is a projection field or it is not shown.
- Overlaps [S-019](S-019-retire-the-flux-connectors-identity.md) in `web/`: `package.json`'s name and
  description and the `PATH_RESOLVER` injection key still say `flux-connectors`. Leave them to S-019
  so the identity moves in one reviewed change.
- Depends on [S-020](S-020-a-ci-gate-exists.md) for the job that runs it; the two should land in one
  wave, since a web job added before the suite is true makes CI red on arrival, and a suite fixed
  without a job is a gate nobody runs.
- `specs/flux/core-v1.json` and the `public/v1/` claim in `web/README.md` go with the core explorer —
  see [S-022](S-022-orphaned-inputs-are-removed-or-re-owned.md).
