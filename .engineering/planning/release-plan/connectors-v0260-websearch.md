---
format: aep.planning-md/3
id: release-plan:connectors-v0260-websearch
kind: release-plan
status: superseded
title: 'Release 0.26.0: generic websearch family with Tavily as its first binding'
relations:
- serves: vision:independent-contract-adapters
- informed_by: epic:generic-websearch
revision: 3
transitions:
- {from: "draft", to: "active", at: "2026-10-05T02:15:38Z", actor: "agent:claude", revision: 2}
- {from: "active", to: "superseded", at: "2026-10-05T06:23:29Z", actor: "agent:claude", revision: 3}
---
## Outcome

Minor release 0.26.0: the provider-independent `datasource.websearch/v1alpha1` family, the
`AuthenticatedHttp::post_json` read capability, and the native `tavily` adapter binding the family
as profile `tavily/2026-10`. It also carries the MCP contract additions merged since 0.25.1
(#80, #82, #83), which are contracts and document checks only.

Minor rather than patch: a new shared family, a new adapter, and a new SDK trait method (with a
default that answers unavailable, so existing adapters compile unchanged).

## Scope

| surface | change |
|---|---|
| `contracts/datasources/websearch/v1alpha1/semantics.md`, `ess/domains/websearch.yaml` | the family and its ESS domain `connectors.websearch` |
| `crates/connectors-sdk`, `crates/connectors-host/src/http.rs`, `contracts/auth/capability/v1alpha1/semantics.md` | `post_json`: fixed paths per composition, 64 KiB body limit, 180 s timeout ceiling |
| `adapters/tavily/` | the adapter, its pinned OpenAPI source, profile document, ESS model and 8 protocol tests |
| `docs/local-tavily.md`, website adapter page, publication entries, status | operator and public documentation |
| `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, `README.md` | version 0.26.0 |

Stories: `story:websearch-contract`, `story:read-post-capability`, `story:tavily-websearch-adapter`
under `epic:generic-websearch`. `story:native-models-in-connectors-namespace` is planned and not
part of this release.

## Evidence

- Local repository gate passed on the code commits before the release-prep commit; the release-prep
  commit changes the version, lockfile workspace entries and documentation only.
- `connectors-build docs --check`: 45 contract pages, 100 reference pages, no drift. Website
  typecheck and production build passed (494 public files, no private path markers).
- Live through the local CLI on 2026-10-05: a Tavily key stored over stdin, `websearch.search` and
  `websearch.crawl` answered, revalidation after the 60 s evidence lifetime. `websearch.fetch` ran
  against the fake server only.
- The local MSRV gate was not rerun after the release-prep commit: the host had under 10G free.
  The PR's `repository gate` CI job is the gate of record for the release commit.

## Completion boundary

The release does not close the MCP runtime work, the adapter model naming story, or the catalog
POST-read story (`story:catalog-post-reads`), which can reuse `post_json` later. Tavily exposes no
account identifier; that limit is stated in the profile and the changelog and is not fixable in the
adapter.
