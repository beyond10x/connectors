---
format: aep.planning-md/3
id: story:connect-names-a-failed-provider-start
kind: story
status: implemented
title: connections connect --endpoint names a provider start that failed instead of refusing the endpoint
summary: When the catalog provider prints its bootstrap but its private child does not start, connect reports the failed start, not invalid_input on endpoint
relations:
- informed_by: story:catalog-provider-failure-is-named
- informed_by: story:macos-release-provider-starts-through-the-private-copy
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-03T22:00:13Z", actor: "human:operator", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-03T22:00:13Z", actor: "human:operator", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-04T08:45:49Z", actor: "human:operator", revision: 4, decided_on: {"recorded":{"test_result":1}}}
---
# Story: connections connect --endpoint names a provider start that failed instead of refusing the endpoint

## Outcome

An operator whose catalog provider prints its bootstrap but cannot be started as a private child learns from
`connections connect <connector>/<instance> --endpoint <url>` that the provider did not start and why, instead of
being told the endpoint is invalid.

## Context

D430 (orchestrator, 2026-11-03), for a downstream adopter on a macOS Apple-silicon laptop with a self-hosted GitLab
at `https://<host>`. On 0.65.0 `connections connect gitlab/<instance> --profile gitlab.pat --endpoint https://<host>`
answers `invalid_input` on `endpoint` at stage `arguments`, while the provider's own `--print-local-bootstrap` at that
endpoint succeeds and every provider start through the host on that machine ends on SIGKILL before the handshake
(`story:macos-release-provider-starts-through-the-private-copy`).

Admitting a new derived instance runs the catalog provider twice (`crates/connectors-host/src/local/config.rs`
`Installation::derive` and `entry`): it prints the bootstrap directly, then starts a `connectors-private/2` child
through the host's private copy (`handshake`, `runtime::Child::spawn`). Any failure of either step reaches
`apps/connectors/src/local/endpoint.rs` `refused_endpoint()`, which answers the endpoint's refusal whenever
`installed_provider_runs()` (a direct `--version`) succeeds. A child that is killed at start is therefore reported as
a refused endpoint. Refines `story:catalog-provider-failure-is-named`.

## Acceptance

- A stand-in catalog provider that prints its version and its bootstrap but ends on SIGKILL when started as a private
  child: `connections connect gitlab/probe --profile gitlab.pat --endpoint <url>` answers `unavailable` at stage
  `installed_provider`, `installed_provider_failure` `killed`, the provider's path named, exit 1, the configuration
  unchanged (`apps/connectors/tests/catalog_provider_failure.rs`).
- The same with a private child that exits non-zero before its handshake: `installed_provider_failure` `exited`.
- An endpoint the running provider refuses (`<host>/api/v4`) is still `invalid_input` on `endpoint` at `arguments`,
  exit 2, nothing written.
- The test is red on `413c60efc` (it answers `invalid_input` on `endpoint`) and green after the change.

## Out of Scope

Why macOS kills the private copy: `story:macos-release-provider-starts-through-the-private-copy`. New refusal fields
or vocabulary values: the existing `installed_provider_failure` values carry the cause.

## Ambiguities

None open. The trailing slash of a printed `provider_authority` (`/api/v4/`) is not compared with the endpoint for a
new instance (`endpoint.rs` `moves_no_connection` compares only with a registered connection's authority), and a
pinned instance of another connector is read, not derived (`config.rs` `InstanceTable::pinned`): neither changes this
refusal (inferable, cited).

## Open Questions

None.
