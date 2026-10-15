---
format: aep.planning-md/3
id: decision-blocker:confluence-token-kind-binding-delivery
kind: decision-blocker
status: open
title: How the Confluence token-kind binding is delivered
relations:
- blocks: story:confluence-token-kind-binding
revision: 1
---
## Question

The Confluence token-kind binding (story:confluence-token-kind-binding) cannot live in the catalog provider alone. How is it delivered?

## Finding

The provider child keeps no state between `validate` and `invoke`, and nothing in the host carries a binding:
- the validation answer (`crates/connectors-host/src/local/runtime.rs:657-664`) and the stored evidence (`registry.rs:232`) are closed records, modelled in `ess/domains/cli.yaml:322` and `ess/domains/credential_evidence.yaml:34`;
- `invoke` carries only operation, revision, partition and deadline (`runtime.rs:690`);
- `operations list` and `describe` use one descriptor per adapter with no connection (`apps/connectors/src/local/operations.rs:55`), while v1 and v2 share ids with different inputs;
- a failed `validate` carrying a reason is a protocol fault (`process.rs:256-263`), so a refusal cannot name both answers.
The files involved overlap the uncommitted lease unit (supervisor, registry, revalidation, cli.yaml, credential_evidence.yaml, CLI semantics).

## Options

a. Host-carried binding: optional `binding` in the private baseline and the stored evidence, optional `bindings` in the bootstrap, optional `binding` on invoke, a reason beside `insufficient_scope` on a failed validate, a rebind field on revalidate, `--connection` on `operations list|describe`; the catalog provider declares alternative bindings. Spec first in shared ESS and contracts. After the lease unit merges. Cost: a host/CLI unit of about the lease unit's size; the Confluence release waits for both.
b. Two steps: release the v1 reads now as their own provider configuration (`confluence-v1`), chosen by the operator's configuration block, with the move steps in the release notes; then deliver (a) as the automatic binding in a later release. Cost: until (a) ships, the caller chooses the API by configuration.
c. Probe on read: the child tries v2 and falls back to v1 on a scope-mismatch 401 on each read. Cost: breaks "probe once", doubles requests for a classic token, and list/describe still show one API.

## Recommendation

b: the operator's token reads Confluence in today's release; (a) follows after the lease unit.
