---
format: aep.planning-md/3
id: review-result:connector-vocabulary-design-r1
kind: review-result
status: active
title: Design critic, round 1, connector-vocabulary stories
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

- story:connector-instance-derived-fields — its acceptance runs `connections connect` on `[connectors.gmail.instances.private]`, an instance with no alias. The host looks up a target only by alias (`config.adapters.get(alias)`), so the command can select it only through `--connector`/`--instance` or `default_instance` resolution, which story:cli-connector-instance-selectors adds. No edge runs from derived-fields to selectors; add `depends_on story:cli-connector-instance-selectors`, which has no path back and so makes no cycle. — .engineering/planning/story/connector-instance-derived-fields.md:70, .engineering/planning/story/cli-connector-instance-selectors.md:52, apps/connectors/src/local.rs:497-503
- story:setup-migrate-adapter-entries — it decides which legacy entries migrate by whether "a connector declares" the implementation, and says the mapping comes from the ESS Provider/Connector/Instance model. The only item in the set that creates connector declarations for catalog providers, with their connector names, is story:catalog-connector-declarations, and no edge or sentence ties the two. Either add `depends_on story:catalog-connector-declarations` or have the body say the mapping is read from somewhere that is not that story's output. — .engineering/planning/story/setup-migrate-adapter-entries.md:49, .engineering/planning/story/catalog-connector-declarations.md:38

What I read: 12 stories, the epic, the ADR, `migration-plan:connector-vocabulary` and `decision-blocker:instance-connection-cardinality`, in full with `aep plan artifact show`. I also ran `aep plan artifact relations`, `graph` and `validate`. I walked 33 `depends_on`, `decomposes`, `informed_by`, `blocks` and `derived_from` edges, including those leaving the set (ADR, migration plan, blocker, epic, `story:upgrade-moves-catalog-adapters`, `story:guarded-write-credential-refusal-says-repair`). `validate` printed only the unrelated review-outcome notices and then `valid`.

Walked edges, and what I found:
- **Cycle:** none in the `depends_on` graph. The longest chain is dual-read, selectors, reconnect, stale, cli-text, docs-readme, then the three docs stories.
- **Chain:** not a serialising chain. The edges in it each have a reason in a body, either an acceptance that reads the earlier story's verb or a shared `cli.yaml` and `local.rs`.
- **Split abstraction:** none. catalog-declarations is demonstrable alone, and derived-fields is its end-to-end reader.
- **Horizontal slice:** none.
- **Acceptance reads traced:** 11 to a producer in the set. Nine are recorded by an edge (directly or through the docs-readme chain). The two findings above are the unrecorded ones.

What I could not establish:
- Out of my lane (parallel safety): reconnect, setup-migrate and selectors all edit `ess/domains/cli.yaml`, `apps/connectors/spec/cli.yaml` and `apps/connectors/src/local.rs`, and reconnect and setup-migrate have no edge between them. cli-text and catalog-declarations both edit `adapters/catalog/src/main.rs` with no edge.
- Out of my lane (scope/acceptance): setup-migrate's acceptance fixture holds credential-binary, MCP, SQL and Kubernetes entries, but derived-fields says those have no connector declaration "until a later story", and no story in the set provides one. It is unclear which of them actually migrate.
- External inputs, not defects: the ESS Provider/Connector/Instance model and the ESS `Stale` change are still being drafted outside the set. I did not inspect their uncommitted state in `ess/`.

```findings
- file: .engineering/planning/story/connector-instance-derived-fields.md
  line: 70
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance runs `connections connect` on an alias-less instance, which the host can select only through the --connector/--instance or default_instance resolution that story:cli-connector-instance-selectors adds, and no depends_on edge runs from it to that story"
- file: .engineering/planning/story/setup-migrate-adapter-entries.md
  line: 49
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it migrates an entry only when a connector declares its implementation, and the connector declarations for catalog providers are created by story:catalog-connector-declarations, with no depends_on edge or sentence naming another source for the mapping"
```
