---
format: aep.planning-md/3
id: release-plan:connectors-v0570-mcp-stdio-list-feeds
kind: release-plan
status: active
title: 'Release 0.57.0: stdio MCP server, connectors list, Jira and Confluence feeds, Retry-After as rate_limited'
relations:
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "active", at: "2026-10-23T16:05:28Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

Minor release 0.57.0 of wave 20261022 (`approval-record:wave-20261022`) and the catalog rate-limit fix (#202):
- `connectors server --transport stdio` (Linux) is a local read-only MCP server over admitted operations.
- `connectors list` prints the connectors the CLI ships a declaration for; `task install` installs both binaries.
- The Jira issue feed and the Confluence REST v1 page feed ship in their selections, lowered from the definitions; Jira gains `projects.list` and `project.get`.
- Under declared `rate_limit_headers`, a `403` or `429` with `Retry-After` is `rate_limited`.
- The first-run documentation path and the upgrade guide.

Minor rather than patch, because:
- there is a new command (`connectors list`) and a new server transport;
- the private protocol gains `connectors-private/3`;
- an answer changes: a `403` with `Retry-After` and requests left was `forbidden` and is now `rate_limited`.

## Scope

| story | status | what remains |
|---|---|---|
| story:definition-reader-feed-extensions | active | four feed shapes the engine refuses at load still lower |
| story:docs-first-run-path | active | — |
| story:mcp-stdio-runtime-branch | active | Linux only; MCP status stays `deferred` |
| story:cli-lists-available-connectors | active | connectors served by their own executables have no row |
| story:catalog-github-reads | active | the GitHub reads themselves; this release carries only the rate-limit rule |

Moving the stories is left to the operator.

## Limitations

These are the limitations the CHANGELOG's 0.57.0 section states:
- The definition reader still lowers four feed shapes the catalog engine refuses at load.
- The stdio MCP server offers no resources, prompts, subscriptions, mutations or outbound MCP.
- A bounded read already at the provider runs to its own budget when the session is lost.
- The Jira and Confluence feeds are verified against fixtures only, not live.

## Evidence

- The repository gate with MSRV ran on 2026-10-23: `connectors-build gate --msrv` with ESS 0.67.0 and aep 0.71.3, on the release tree (main `25ed378a2` with #203 merged in, plus the version, CHANGELOG and getting-started edits).
  - 30 steps, all exit 0.
  - 3142 tests passed, 0 failed, 148 ignored.
  - The gate printed "gate: all checks passed".
- After the gate, `apps/connectors/tests/derived_instance.rs` gained an ETXTBSY retry on the CLI launch (main's Rust gate on `25ed378a2` failed once at line 72 with "Text file busy"). On the final tree `cargo test -p connectors --test derived_instance` passed 3 of 3 and its clippy with `-D warnings` was clean.
- `target/debug/connectors --version` printed `connectors 0.57.0`.
