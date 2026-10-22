---
format: aep.planning-md/3
id: story:jira-feed-binding
kind: story
status: active
title: Jira binds the feed family
relations:
- decomposes: epic:generic-datasource-feeds
- serves: vision:independent-contract-adapters
- depends_on: story:feed-contract
- depends_on: story:feed-bindings-discoverable
- depends_on: story:catalog-feed-engine
- depends_on: story:catalog-feed-engine-extensions
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: adapters/catalog/contracts/feed/v1alpha1/jira.md
- confidence: cited
  path: adapters/catalog/providers/jira/operations.json
- confidence: cited
  path: adapters/catalog/tests/connector_declarations.rs
- confidence: inferred
  path: adapters/catalog/tests/feed/jira/
- confidence: cited
  path: adapters/catalog/tests/gateway_prefix.rs
- confidence: inferred
  path: adapters/catalog/tests/jira_feed.rs
- confidence: inferred
  path: crates/connectors-build/src/jira_feed_conformance.rs
- confidence: inferred
  path: crates/connectors-build/src/main.rs
- confidence: inferred
  path: crates/connectors-docs/src/status.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T19:55:09Z", actor: "agent:claude", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-06T19:55:09Z", actor: "agent:claude", revision: 3}
---
## Outcome

The Jira adapter binds `datasource.feed/v1alpha1`: its containers and items are readable through the family, with the binding's own profile, under `adapters/catalog/contracts/feed/v1alpha1/`.

## Acceptance

The family's conformance suite passes against the binding with recorded provider fixtures, and the binding's semantics state: what a container is (a project), how the watermark is formed, how far a first read reaches, whether deletions are observed, and how direct or private material is excluded.

## Depends on

`story:feed-contract`, `story:feed-bindings-discoverable`.

## Scope

Derived 2026-10-21 by `story-scoper` at e26481df09. Every line is **cited** (read from the story or the tree) or **inferred**.

- **Primary surface:** `adapters/catalog/providers/jira/operations.json` (the `feed` block) — cited
- **Binding files (new):** `adapters/catalog/contracts/feed/v1alpha1/jira.md`, `adapters/catalog/tests/jira_feed.rs`, `adapters/catalog/tests/feed/jira/{project,issue}.json`, `crates/connectors-build/src/jira_feed_conformance.rs`, its mod line in `crates/connectors-build/src/main.rs` — inferred (GitLab pattern)
- **Pins that move:** `adapters/catalog/tests/gateway_prefix.rs:171-172`, `adapters/catalog/tests/connector_declarations.rs:736,740,744` — cited
- **Pins that stay:** `adapters/catalog/generated/bundles/jira.bundle.json` (the three feed operations `searchProjects`, `getProject`, `searchAndReconsileIssuesUsingJql` are already in it), `adapters/catalog/providers/jira/connector.json` — cited
- **Upstream:** `searchProjects` (`GET /rest/api/3/project/search`): `startAt`, `maxResults` (max 100), answers `isLast`, `values`; `searchAndReconsileIssuesUsingJql` (`/rest/api/3/search/jql`): bounded `jql`, `fields` defaults to `id`, answers `issues`, `isLast`, `nextPageToken` (`adapters/atlassian/upstream/jira-platform-v3.json`) — cited
- **Engine items needed:** container offset paging, the JQL template, `±hhmm` and minute precision, the time-list body-token continuation, all from `story:catalog-feed-engine-extensions` — cited
- **Also likely:** `CHANGELOG.md`, `crates/connectors-docs/src/status.rs` and its regenerated pages — inferred
- **Would collide with:** `story:catalog-feed-engine-extensions` (lands first)
- **Safety fact:** a `feed` enters a configuration's revision only when declared (`adapters/catalog/src/local.rs:1173-1175`), so no other provider's pins move — cited, unproven by a run

### Decisions taken by the wave coordinator (2026-10-21)

| question | decision | reason |
|---|---|---|
| visibility | every project reads as private | GitLab's rule; a mapped `isPrivate` would widen what a consumer may show, and no live Jira read checks it |
| container kind | the fixed word `project` | `projectTypeKey` values are not enumerated in the pinned upstream |
| item body | the issue `summary` | `description` is ADF, which §3.3 makes unavailable; `renderedFields` is HTML |
| container reach | the upstream defaults (`status=live`, `action=view`) and a declared `updated` time clause | archived and unreadable projects stay out; the first read starts at the watermark |

Not established: the live response shapes (no recorded Jira feed fixture; shapes come from the upstream spec and `adapters/catalog/tests/jira.rs`); the index lag bound of `/search/jql`. The status row says fixtures only.
