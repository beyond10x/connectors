---
format: aep.planning-md/3
id: story:expired-evidence-invoke-advises-revalidate
kind: story
status: draft
title: An invoke on a connection whose evidence expired advises revalidate, not repair
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:connector-probe-20261006
- depends_on: story:service-failure-carries-upstream-reason
- depends_on: story:dependency-refresh-20261006
scope:
- confidence: inferred
  path: apps/connectors-cli-contract/binding.json
- confidence: cited
  path: apps/connectors/src/local.rs
- confidence: cited
  path: apps/connectors/src/local/connections.rs
- confidence: cited
  path: contracts/auth/evidence/v1alpha1/semantics.md
- confidence: cited
  path: contracts/cli/v1alpha1/scenarios.md
- confidence: cited
  path: contracts/cli/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-host/src/local/owner.rs
- confidence: cited
  path: docs/local-catalog-provider.md
- confidence: inferred
  path: ess/domains/cli.yaml
revision: 6
---
## Observed

2026-09-30, live Jira through the Atlassian gateway (service account, basic auth, connectors 0.18.0 plus the gateway
unit): a connection that answered `ready` at connect was `pending` a few minutes later; `operations invoke` then
answered `not_granted` / `repair_connection` at `admission`. `connections revalidate` (no credential re-entry) brought it
back and the same invoke returned 10 issues. The knowledge-ingest consumer reported the same on 0.15.1 ("an invoke
returns not_granted / repair_connection rather than prompting a revalidate, while connections list says pending").

## Acceptance

Decided 2026-10-06: the invoke does not revalidate on its own (`docs/local-runtime-foundation.md`:
"implicit identity probes and credential repair are not part of a business read"); it names the
action that helps.

- `connectors.cli.NextAction` (`ess/domains/cli.yaml:301-303`) gains the variant
  `revalidate_connection`, and the CLI contract (`contracts/cli/v1alpha1/semantics.md`,
  `scenarios.md`, the generated `apps/connectors-cli-contract`) states it.
- An invoke refused at `admission` with `not_granted` only because the connection's validation
  evidence expired, while its credential is intact, answers `next_action: revalidate_connection`;
  the owner carries that reason distinctly from an insufficient scope
  (`crates/connectors-host/src/local/owner.rs`, where `NotReady` and `InsufficientScope` both map to
  `NotGranted` today). An insufficient scope keeps its current answer.
- A test runs an invoke after the evidence lifetime and asserts `revalidate_connection`; `connections
  revalidate` then makes the same invoke succeed.
- `docs/local-catalog-provider.md` states the evidence lifetime of catalog providers (60 s unless
  `evidence_lifetime_ms`) and the revalidate step.

## Ordering

Shares `apps/connectors/src/local.rs`, `crates/connectors-host/src/local/owner.rs`,
`contracts/cli/v1alpha1/semantics.md`, `ess/domains/cli.yaml` and the generated CLI contract with
`story:service-failure-carries-upstream-reason`, and `docs/local-catalog-provider.md` with
`story:catalog-honours-retry-after` and `story:forge-issue-create`; it lands after
`story:service-failure-carries-upstream-reason` (`depends_on`) and after
`story:dependency-refresh-20261006`, which regenerates the CLI contract and `ess/`.
