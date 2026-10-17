---
format: aep.planning-md/3
id: review-result:connector-vocabulary-parallel-r1
kind: review-result
status: active
title: Parallel-safety critic, round 1, connector-vocabulary stories
relations:
- reviews: story:catalog-connector-declarations
- reviews: story:cli-connector-instance-selectors
- reviews: story:cli-text-without-adapter
- reviews: story:connections-reconnect-verb
- reviews: story:connector-config-dual-read
- reviews: story:connector-instance-derived-fields
- reviews: story:connection-stale-state
- reviews: story:setup-migrate-adapter-entries
- reviews: story:docs-catalog-guides-vocabulary
- reviews: story:docs-google-one-instance-per-account
- reviews: story:docs-local-guides-vocabulary
- reviews: story:docs-vocabulary-readme-website
revision: 1
---
needs-revision
story:setup-migrate-adapter-entries — it and story:connections-reconnect-verb both land on `ess/domains/cli.yaml`, `apps/connectors/spec/cli.yaml`, `apps/connectors/src/local.rs`, `contracts/cli/v1alpha1/semantics.md`, `apps/connectors/tests/` and the regenerated `apps/connectors-cli-contract` (cited, both Scope sections), no `depends_on` edge joins them and neither body mentions the other, so record the shared files as an ordering edge in one direction or split the surface — .engineering/planning/story/setup-migrate-adapter-entries.md:69-73, .engineering/planning/story/connections-reconnect-verb.md:59-63
story:setup-migrate-adapter-entries — it and story:connector-instance-derived-fields both edit `crates/connectors-host/src/local/config.rs` (cited: "serialising the new form" against "derivation of the `Adapter` value"), both depend on story:connector-config-dual-read, neither depends on the other and neither body names the shared file, so record the file as an ordering edge or split the two changes into separate surfaces — .engineering/planning/story/setup-migrate-adapter-entries.md:70, .engineering/planning/story/connector-instance-derived-fields.md:62
story:cli-text-without-adapter — it and story:catalog-connector-declarations both land on `adapters/catalog/src/main.rs` (cited: "(help)" against "(argument parsing)"), the help rewrite describes the arguments the other story adds, and there is no `depends_on` edge and no mention either way, so name an edge from this story to the declarations story or split the file's surface — .engineering/planning/story/cli-text-without-adapter.md:65, .engineering/planning/story/catalog-connector-declarations.md:65
story:cli-text-without-adapter — its `crates/connectors-host/src/local/**` scope covers `config.rs` and `runtime.rs`, which story:connector-instance-derived-fields edits (cited, both Scope sections), and none of this story's four `depends_on` edges reaches that story (only the downstream docs story joins them), so name an ordering edge recording the shared files or narrow the sweep to files that story does not touch — .engineering/planning/story/cli-text-without-adapter.md:64, .engineering/planning/story/connector-instance-derived-fields.md:62-63

What I read: 12 stories (`aep plan artifact show` on each), `aep plan artifact waves --kind story --status draft`, `aep plan artifact graph`, `aep plan artifact relations`, plus a `git`/`ls` check of the test directories in the worktree.

Surfaces: 12 placed from body Scope sections (cited), 0 inferred, 0 unplaceable. The store marks every scope entry `inferred`, but each body names its paths itself. Some named paths do not exist yet: `adapters/catalog/providers/*/` declarations, a possible `adapters/catalog/build.rs`, and a possible `apps/connectors/src/local/migrate.rs`.
Acceptance reads traced to a producer in the set: 21, of which 20 have a `depends_on` edge, directly or through `docs-vocabulary-readme-website`. The 21st is the text story's help reading the new `adapters/catalog/src/main.rs` arguments, reported above.

What I could not establish:
- **External custody overlap.** The OpenRouter custody unit changes `config.rs`, `ess/domains/cli.yaml` and `docs/local-mutation-ledger.md`. Only story:connector-config-dual-read names it ("rebase on that unit before implementing"). The other stories that touch those files all sit downstream of it through `depends_on` (the docs-local story via docs-vocabulary and derived-fields), so I raised no finding. This holds only if the unit lands on main before the dual-read story starts.
- **Stale in `ess/domains/auth_bindings.yaml`.** story:connection-stale-state reads it as an external input, and its Scope lists `entity-runtime-definitions.json`, which that ESS change also regenerates. I could not see that change, so I cannot say whether the two collide.
- **Shared test directories.** Five stories claim `apps/connectors/tests/` (selectors, reconnect, setup-migrate, text, derived-fields) and the derived-fields and stale stories both claim `adapters/catalog/tests/local_runtime`. I found no shared file, only directory-level overlap, and the new journey files look distinct. The derived-fields story would also have to edit `adapters/catalog/tests/local_runtime.rs` to register a new module, and its Scope omits that file. That is not a collision on its own.
- **Setup-migrate's source of connector names.** Its migration of catalog entries needs the connector name for each implementation. Its body says "from the ESS model", which is not in the set, while the declarations story also ships connector names. I could not tell whether it reads the declarations story's output.
- **Outside the 12.** `aep plan artifact waves` also lists collisions with stories outside the set (catalog-google-live-deck-read, catalog-zendesk-reads, configuration-change-error, catalog-validates-enum-and-date-time). I did not assess them.
- **Out of my lane.** The `Scope` of story:cli-text-without-adapter spans whole subtrees (`apps/connectors/src/**`, `crates/connectors-host/src/local/**`), which may be too wide. That is a design-critic matter, and I did not let it set the verdict.

```findings
- file: .engineering/planning/story/setup-migrate-adapter-entries.md
  line: 69
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "this story and story:connections-reconnect-verb both land on ess/domains/cli.yaml, apps/connectors/spec/cli.yaml, apps/connectors/src/local.rs, contracts/cli/v1alpha1/semantics.md, apps/connectors/tests and the regenerated apps/connectors-cli-contract, with no depends_on edge and no mention in either body; record the shared files as an ordering edge or split the surface (surfaces cited from both Scope sections)"
- file: .engineering/planning/story/setup-migrate-adapter-entries.md
  line: 70
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "this story (serialising the new form) and story:connector-instance-derived-fields (derivation of the Adapter value) both edit crates/connectors-host/src/local/config.rs, both depend on story:connector-config-dual-read, neither depends on the other and neither body names the shared file; record an ordering edge or split the surface (cited from both Scope sections)"
- file: .engineering/planning/story/cli-text-without-adapter.md
  line: 65
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "this story's help rewrite and story:catalog-connector-declarations' argument parsing both land on adapters/catalog/src/main.rs, the help describes the arguments the other story adds, and there is no depends_on edge or mention; name an edge to story:catalog-connector-declarations or split the surface (cited from both Scope sections)"
- file: .engineering/planning/story/cli-text-without-adapter.md
  line: 64
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the crates/connectors-host/src/local/** scope covers config.rs and runtime.rs that story:connector-instance-derived-fields edits (cited, both Scope sections), and none of this story's depends_on edges reaches that story; name an ordering edge recording the shared files or narrow the sweep"
```
