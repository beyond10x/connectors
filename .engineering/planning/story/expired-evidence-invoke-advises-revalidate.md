---
format: aep.planning-md/3
id: story:expired-evidence-invoke-advises-revalidate
kind: story
status: draft
title: An invoke on a connection whose evidence expired advises revalidate, not repair
relations:
- serves: vision:independent-contract-adapters
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
  path: crates/connectors-host/src/local/owner/mutation/recovery.rs
- confidence: inferred
  path: crates/connectors-host/src/local/registry/observation.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/tests.rs
- confidence: cited
  path: crates/connectors-host/src/local/registry/use_and_retirement.rs
- confidence: cited
  path: docs/local-catalog-provider.md
- confidence: inferred
  path: ess/domains/cli.yaml
revision: 3
---
## Observed

2026-09-30, live Jira through the Atlassian gateway (service account, basic auth, connectors 0.18.0 plus the gateway
unit): a connection that answered `ready` at connect was `pending` a few minutes later; `operations invoke` then
answered `not_granted` / `repair_connection` at `admission`. `connections revalidate` (no credential re-entry) brought it
back and the same invoke returned 10 issues. The knowledge-ingest consumer reported the same on 0.15.1 ("an invoke
returns not_granted / repair_connection rather than prompting a revalidate, while connections list says pending").

## Acceptance

- An invoke on a connection whose evidence expired, but whose credential is intact, answers with next action
  `retry_status` or a dedicated revalidate action as the contract names, never `repair_connection` (which asks for a
  new credential).
- Or: invoke revalidates transparently when only the evidence expired, if the contract allows it; the story decides
  which and records it in semantics.md.
- How long evidence stays valid for catalog providers is stated in the docs.
