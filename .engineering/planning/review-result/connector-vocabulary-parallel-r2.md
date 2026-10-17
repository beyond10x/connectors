---
format: aep.planning-md/3
id: review-result:connector-vocabulary-parallel-r2
kind: review-result
status: active
title: Parallel-safety critic, round 2, connector-vocabulary stories
relations:
- reviews: story:catalog-connector-declarations
- reviews: story:cli-connector-instance-selectors
- reviews: story:cli-text-without-adapter
- reviews: story:connection-stale-state
- reviews: story:connections-reconnect-verb
- reviews: story:connector-config-dual-read
- reviews: story:connector-instance-derived-fields
- reviews: story:docs-catalog-guides-vocabulary
- reviews: story:docs-google-one-instance-per-account
- reviews: story:docs-google-publishing-advice
- reviews: story:docs-local-guides-vocabulary
- reviews: story:docs-pinned-catalog-guides-vocabulary
- reviews: story:docs-vocabulary-readme-website
- reviews: story:ess-connector-vocabulary-model
- reviews: story:setup-migrate-adapter-entries
revision: 1
---
approve

**What I read:** 15 stories (every story with `decomposes:epic:connector-vocabulary`), read with `aep plan artifact show` on each. I also ran `aep plan artifact waves --kind story --status draft`, `aep plan artifact graph` for the `depends_on` edges, `aep plan artifact relations`, and `review-result:connector-vocabulary-parallel-r1`. I spot-checked the tree with `ls` and `git grep` under `apps/connectors/tests`, `adapters/catalog/tests`, `ess/domains/cli.yaml` and `crates/connectors-docs/src`.

**Round 1.** All four round-1 findings are fixed by edges now in the graph:
- `story:setup-migrate-adapter-entries` depends on `story:connections-reconnect-verb` and `story:connector-instance-derived-fields`.
- `story:cli-text-without-adapter` depends on `story:catalog-connector-declarations` and `story:connector-instance-derived-fields`.

`config.rs`, `ess/domains/cli.yaml`, `apps/connectors/spec/cli.yaml`, `apps/connectors/src/local.rs`, `contracts/cli/v1alpha1/semantics.md`, `apps/connectors-cli-contract` and `adapters/catalog/src/main.rs` are each touched by several stories. Every pair that shares one of these files is ordered by a direct `depends_on` edge or a chain of them. The `waves` collisions between members of the set are all ordered this way. The three stories new since round 1 (`story:ess-connector-vocabulary-model`, `story:docs-google-publishing-advice`, `story:docs-pinned-catalog-guides-vocabulary`) add no unordered pair. The five docs stories have disjoint file sets and all depend on `story:docs-vocabulary-readme-website`.

**Surfaces:** 15 cited, 0 inferred, 0 unplaceable. The store marks every scope entry `inferred`, but each body names its own paths. Some named files do not exist yet: the per-connector declaration files under `adapters/catalog/providers/`, a possible `adapters/catalog/build.rs`, a possible `apps/connectors/src/local/migrate.rs`, and the new Gmail journey file in `adapters/catalog/tests/local_runtime/`. No two stories create the same new file.

**Acceptance reads:** 28 traced to a producer in the set, and all 28 are recorded by a `depends_on` edge. 23 are direct. Five are direct to `story:docs-vocabulary-readme-website`, which adds the `vocabulary` check, and reach the code stories through it.

**What I could not establish:**
- **External OpenRouter custody unit.** Only `story:connector-config-dual-read` names it ("rebase on that unit before implementing"). Every story that touches `config.rs`, or `docs/local-mutation-ledger.md` via `story:docs-local-guides-vocabulary`, sits downstream of it by edge, so I raised no finding. `story:ess-connector-vocabulary-model` is different: it is a root story with no incoming edge, it edits the header comment of `ess/domains/cli.yaml`, and it regenerates `entity-runtime-definitions.json`, which has ten `connectors.cli` entries. Its body does not name the unit. This is safe only if the unit is on main before that story is branched. If both run at once, the regenerated JSON is the likely conflict.
- **`apps/connectors/tests/` overlap with no ordering edge.** `story:connections-reconnect-verb` and `story:connector-instance-derived-fields` both cite the directory, and no `depends_on` edge joins them. I found no shared file: reconnect edits existing tests that use `repair` or `adapters`, such as `cli_surface_minor.rs`, `failed_connect.rs` and `json_answers.rs`, while derived-fields adds a no-pins configuration test. That is directory-level and I reported no finding. `waves` placed them in separate waves (5 and 6) because of it.
- **Stories outside the set.** `waves` reports collisions between these stories and stories outside the set: `docs/local-catalog-provider.md` with `story:catalog-zendesk-reads`, `story:catalog-validates-enum-and-date-time` and `story:configuration-change-error`, `ess/domains/cli.yaml` with `story:configuration-change-error`, and `ess/domains/declarations.yaml` with `story:kubernetes-spec-service`. I did not assess these.
- **Out of my lane (scope critic).** `story:ess-connector-vocabulary-model` may add a `Stale` variant to `ConnectionState`, which `ConnectionSummary.state` in `ess/domains/cli.yaml:706` uses. `story:connector-config-dual-read` may add a name to `ConfigurationRule`. Neither story lists `apps/connectors-cli-contract` as regenerated, so `cargo run --locked -p connectors-build -- cli --check` would fail until `story:cli-connector-instance-selectors` regenerates it. That is an ordering fact, not a collision.

```findings
[]
```
