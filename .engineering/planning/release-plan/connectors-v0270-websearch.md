---
format: aep.planning-md/3
id: release-plan:connectors-v0270-websearch
kind: release-plan
status: active
title: 'Release 0.27.0: generic websearch family with Tavily as its first binding'
relations:
- serves: vision:independent-contract-adapters
- informed_by: epic:generic-websearch
- supersedes: release-plan:connectors-v0260-websearch
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-05T06:23:29Z", actor: "agent:claude", revision: 2}
---
## Outcome

Minor release 0.27.0: the provider-independent `datasource.websearch/v1alpha1` family, the
`AuthenticatedHttp::post_json` read capability, and the native `tavily` adapter binding the family
as profile `tavily/2026-10`.

This scope was first recorded as 0.26.0 (`release-plan:connectors-v0260-websearch`). Main released
0.26.0 for the Zendesk catalog reads and the MCP contracts (`release-plan:connectors-v0260-zendesk-reads`)
while PR #85 was in CI, so this release moved to 0.27.0 and its changelog no longer lists the MCP
items.

Minor rather than patch: a new shared family, a new adapter, and a new SDK trait method (with a
default that answers unavailable, so existing adapters compile unchanged).

## Scope

| surface | change |
|---|---|
| `contracts/datasources/websearch/v1alpha1/semantics.md`, `ess/domains/websearch.yaml` | the family and its ESS domain `connectors.websearch` |
| `crates/connectors-sdk`, `crates/connectors-host/src/http.rs`, `contracts/auth/capability/v1alpha1/semantics.md` | `post_json`: fixed paths per composition, 64 KiB body limit, 180 s timeout ceiling |
| `adapters/tavily/` | the adapter, its pinned OpenAPI source, profile document, ESS model and 8 protocol tests |
| `docs/local-tavily.md`, website adapter page, publication entries, status | operator and public documentation |
| `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, `README.md` | version 0.27.0 |

Stories: `story:websearch-contract`, `story:read-post-capability`, `story:tavily-websearch-adapter`
under `epic:generic-websearch`. `story:native-models-in-connectors-namespace` is planned and not
part of this release.

## Evidence

- PR #85 at `93e9c6fad` (before the merge of 0.26.0): `repository gate`, `planning validate`,
  `b10x-docs-check` and `Security and privacy` passed; repository gate 19m18s,
  https://github.com/beyond10x/connectors/actions/runs/37254769653.
- After merging main: `connectors-build docs --check` (no drift) and `cargo check` of
  `connectors-tavily` and `connectors-host`. The PR's checks rerun on the merged head and are the
  gate of record for the release commit.
- Live through the local CLI on 2026-10-05: a Tavily key stored over stdin, `websearch.search` and
  `websearch.crawl` answered, revalidation after the 60 s evidence lifetime. `websearch.fetch` ran
  against the fake server only.

## Completion boundary

The release does not close the adapter model naming story or the catalog POST-read story
(`story:catalog-post-reads`), which can reuse `post_json` later. Tavily exposes no account
identifier; that limit is stated in the profile and the changelog and is not fixable in the
adapter.
