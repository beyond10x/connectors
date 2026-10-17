---
format: aep.planning-md/3
id: review-result:connector-vocabulary-design-r2
kind: review-result
status: active
title: Design critic, round 2, connector-vocabulary stories
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
needs-revision

- story:docs-pinned-catalog-guides-vocabulary — its premise that Slack is "a credential binary" that derived-fields does not derive contradicts the set. Slack is a catalog provider (`adapters/catalog/providers/slack/` exists; `docs/catalog-slack.md:341-355` configures `"provider": "slack"`). catalog-connector-declarations ships one declaration per catalog provider directory, so it ships `slack`. derived-fields derives any pinless catalog connector instance, and excludes only "credential binaries, MCP, SQL, Kubernetes". Nothing in the set excludes Slack, so either the body says why `slack` is pinned (for example, the bot and user profiles select different operation sets, while derived-fields gives an instance every operation of its connector) and catalog-connector-declarations or derived-fields records that exclusion, or the Slack guide moves to the derived form and into story:docs-catalog-guides-vocabulary. — .engineering/planning/story/docs-pinned-catalog-guides-vocabulary.md:20-21, .engineering/planning/story/catalog-connector-declarations.md:39, .engineering/planning/story/connector-instance-derived-fields.md:38-50

What I read: 15 stories (`decomposes epic:connector-vocabulary`), whole bodies, plus `review-result:connector-vocabulary-design-r1`, `decision-blocker:instance-connection-cardinality`, and the repository files cited above. Commands: `aep plan artifact relations`, `list`, `graph` (the full graph, not just the set), `waves` and `validate` (it printed only unrelated review-outcome notices, then `valid`). I walked 66 edges (every `depends_on`, `decomposes`, `informed_by`, `blocks` and `derived_from` among the 15, plus the ADR, migration plan, blocker and epic); the 3 `informed_by` edges to stories outside the set (upgrade-moves-catalog-adapters, guarded-write-credential-refusal-says-repair, plus the implemented oauth2-refresh) need no ordering.

- **Round-1 findings:** both are closed. `connector-instance-derived-fields` now has `depends_on story:cli-connector-instance-selectors`. `setup-migrate-adapter-entries` now has `depends_on story:catalog-connector-declarations` and a body sentence naming the source of the mapping.
- **Cycle:** none in `depends_on`. Topological order is model; then catalog-declarations and dual-read; then selectors; then reconnect and derived-fields; then stale-state and setup-migrate; then cli-text; then docs-readme; then the five guide stories.
- **Chain:** the long path (model → dual-read → selectors → reconnect → setup-migrate → cli-text → readme → guides) is not a finding, because each edge has a stated reason in a body (an acceptance that reads the earlier verb, or a shared file).
- **Split abstraction and horizontal slice:** none. The model story is a foundation that validates alone, and every other code story has its own end-to-end acceptance.
- **Acceptance reads traced:** 17 to a producer in the set. All 17 are recorded by an edge, directly or through docs-readme.

What I could not establish:
- Out of my lane (parallel safety): `ess/domains/cli.yaml`, `apps/connectors/spec/cli.yaml` and `local.rs` are edited by selectors, reconnect and setup-migrate. These are chained by edges, but reconnect → setup-migrate and catalog-declarations → cli-text on `adapters/catalog/src/main.rs` are recorded only transitively or by `depends_on`. I did not check whether `waves` places `connection-stale-state` and other items beside colliding files outside the set.
- Out of my lane (scope): migration-plan step 9 (removal) has no story in the set, and the catalog guides rely on Gmail-only fixture evidence for Jira, Confluence and HubSpot (credential blockers open).
- Not inspected: the uncommitted `ess/` drafts. The model story's acceptance depends on them.

```findings
- file: .engineering/planning/story/docs-pinned-catalog-guides-vocabulary.md
  line: 20
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it treats Slack as a credential binary that is not derived, but Slack is a catalog provider with a declaration under story:catalog-connector-declarations (one per adapters/catalog/providers/<name>/) and story:connector-instance-derived-fields derives every pinless catalog connector instance, so no item in the set excludes slack; either say why slack stays pinned and record that exclusion in catalog-connector-declarations or derived-fields, or move the Slack guide to the derived form"
```
