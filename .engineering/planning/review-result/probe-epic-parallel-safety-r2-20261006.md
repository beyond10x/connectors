---
format: aep.planning-md/3
id: review-result:probe-epic-parallel-safety-r2-20261006
kind: review-result
status: active
title: Parallel-safety critic, round 2, epic:connector-probe-20261006
relations:
- reviews: story:dependency-refresh-20261006
- reviews: story:registry-clock-floor-growth
- reviews: story:owner-memory-bounded
- reviews: story:cli-json-answers-as-json
- reviews: story:upgrade-adapter-identity-mismatch-names-new-connection
- reviews: story:forge-issue-create
- reviews: story:expired-evidence-invoke-advises-revalidate
- reviews: story:catalog-honours-retry-after
- reviews: story:service-failure-carries-upstream-reason
- reviews: story:metadata-invoke-cost-flat-in-store-size
revision: 1
---
needs-revision

- `story:cli-json-answers-as-json` — it shares `contracts/cli/v1alpha1/semantics.md` and `ess/domains/cli.yaml` (cited in its scope and in `story:service-failure-carries-upstream-reason`), and `semantics.md` with `story:expired-evidence-invoke-advises-revalidate` (cited), yet its "Would collide with" line names only a generic class ("any unit touching `local/operations.rs`, the `connectors.cli` types…"). Its only `depends_on` is dependency-refresh, and expired's Ordering omits cli-json. The order service-failure, then cli-json, then expired exists only as derived waves 3, 6 and 7. Name the pairs in an Ordering section and add `depends_on` edges, or split the contract and `cli.yaml` edits — `.engineering/planning/story/cli-json-answers-as-json.md:84`
- `story:service-failure-carries-upstream-reason` — it edits the generated CLI contract (`apps/connectors-cli-contract/binding.json`, cited) and `ess/domains/cli.yaml` (cited), both inside `story:dependency-refresh-20261006`'s scope (`apps/connectors-cli-contract` and `ess/domains`, inferred on that side). Dependency-refresh regenerates them and says every such unit goes after it. Unlike cli-json, registry-clock and expired, this story has no `depends_on` edge to it and no Ordering section. `aep plan artifact waves` matches exact paths and does not list this pair, so wave 3 is not a guarantee. Add the edge or an Ordering section, or split the surface — `.engineering/planning/story/service-failure-carries-upstream-reason.md:16`

**What I read:** all 10 stories with a `decomposes` edge to the epic, plus the epic and review-result r1. Commands: `aep plan artifact show` on each, `aep plan artifact waves --kind story --status draft --format json` (I filtered its collisions to the set), and `git grep`/`grep` on the story files.

**Surface placement:** 10 cited, 0 wholly inferred, 0 unplaceable. Several code sites are still only inferred, such as the retry-after sites in `adapters/catalog/src/lib.rs`, `owner.rs` and `runtime.rs`, and forge's test files.

**Round-1 findings:**
- **Fixed:**
  - expired × service-failure now has an Ordering section and a `depends_on` edge.
  - Expired's scope is narrowed to 9 files and its fix site is decided.
  - Expired no longer touches `registry/tests.rs`.
  - retry-after × service-failure has a `depends_on` edge and its Ordering names `lib.rs:895-908`.
  - The `docs/local-catalog-provider.md` overlap of retry-after with expired and forge is now named, and derived waves 4, 5 and 7 keep them apart.
- **Open:** none of the five round-1 findings holds. The two findings above are new.

**What I could not establish:**
- **`story:metadata-invoke-cost-flat-in-store-size`:** it is `active` and has no recorded blocker edge, only prose naming entity-runtime#55. It shares `metadata/er.rs`, `registry.rs`, `registry/tests.rs` and `Cargo.*` with registry-clock, owner-memory and dependency-refresh. Only owner-memory names it. It is not in the waves, so I treated it as non-concurrent, as in round 1.
- **Order recorded only in prose or derived waves, accepted:**
  - owner-memory × registry-clock (`metadata/er.rs`, `store_cost_tests.rs`) is named in owner-memory only.
  - upgrade-mismatch × expired and service-failure (`local.rs`) is named in upgrade-mismatch only.
  - owner-memory × dependency-refresh (`Cargo.*`, inferred) is covered by dependency-refresh's generic rule.
  - forge × cli-json (`CHANGELOG.md`, inferred both sides) is additive.
- **Out of my lane:**
  - Several bodies carry a duplicated `## Acceptance` heading (design or acceptance critic).
  - Whether cli-json should split `OperationDescription` (design).

```findings
- file: .engineering/planning/story/cli-json-answers-as-json.md
  line: 84
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: shares contracts/cli/v1alpha1/semantics.md and ess/domains/cli.yaml (cited) with story:service-failure-carries-upstream-reason, and semantics.md (cited) and cli.yaml (inferred in expired's scope) with story:expired-evidence-invoke-advises-revalidate, but names only a generic class, has no Ordering section, and its only depends_on is dependency-refresh, so the order exists only in derived waves; name the pairs and add depends_on edges, or split the surface
- file: .engineering/planning/story/service-failure-carries-upstream-reason.md
  line: 16
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: edits apps/connectors-cli-contract/binding.json and ess/domains/cli.yaml (cited), which lie inside story:dependency-refresh-20261006's scope (apps/connectors-cli-contract and ess/domains, inferred on that side) and which that story regenerates; unlike cli-json, registry-clock and expired it has no depends_on edge or Ordering section, and the waves tool matches exact paths so it does not see the pair; add the edge or an Ordering section, or split the surface
```
