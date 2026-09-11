---
format: aep.planning-md/1
id: story:persistent-kubernetes-journey
kind: story
status: implemented
title: Persist and reuse an admitted Kubernetes connection through the local CLI
relations:
- decomposes: initiative:complete-local-connectors
- informed_by: story:persistent-gitlab-journey
- informed_by: story:local-cli-binding-semantics
- informed_by: story:three-adapters-e2e
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: adapters/kubernetes
- confidence: cited
  path: apps/connectors
- confidence: cited
  path: docs
revision: 5
---
## Acceptance

From a fresh private configuration on Linux x86_64, an owner configures a Kubernetes adapter entry, enters an existing cluster credential through a protected source, saves a validated connection, invokes each admitted Kubernetes read, restarts both the CLI and the local owner, and repeats the reads using the retained exact credential version without re-entry. Namespace and resource-kind refusals, bounded continuation and optional host discovery stay distinguishable from empty success.

## Existing semantic owners

contracts/cli/v1alpha1/semantics.md, contracts/auth/management.md and the connection, acquisition, custody and evidence contracts own admission, protected entry, publication and reuse; they are already implemented generically in crates/connectors-host and apps/connectors for the GitLab journey and carry no provider vocabulary.

adapters/kubernetes/contracts/auth/v1alpha1/semantics.md owns the native binding. It selects SelfSubjectReview as the identity-validation probe and fixes the SelfSubjectAccessReview target tuple, fan-out budget and authorization-coverage payload for per-operation permission checks. adapters/kubernetes/contracts/discovery/v1alpha1/semantics.md and contracts/reads own the existing three operations. adapters/kubernetes/spec/ess owns the native typed model.

## Sequence and boundaries

1. Add an executable composition module to adapters/kubernetes, in the shape adapters/gitlab/src/local.rs already establishes: a `connectors-kubernetes-local/1` owner-only native configuration file, `--local-config` and `--print-local-bootstrap` on the adapter binary, a declared native credential profile and a validated `Bootstrap` with one requirement per advertised operation.
2. Terminate native configuration and protected entry in that module. The business `Kubernetes` adapter keeps receiving only an immutable authenticated HTTP port; it gains no database, keyring or arbitrary-POST authority.
3. Implement identity validation through the contract's selected SelfSubjectReview probe under a private check-specific capability. The generic read capability grants no POST authority, and ordinary business reads cannot invent the probe.
4. Bind the existing `resources.list`, `endpoints.discover` and `hosts.discover` operations to the local owner's dispatch with their current scope, paging, optional-host and provenance semantics unchanged.
5. Write deterministic failure and restart tests covering configured-namespace refusal, unsupported resource kind, cursor continuation, custody unavailability, owner restart reuse, same-identity repair and terminal revocation.
6. Document the journey in a Kubernetes counterpart to docs/local-gitlab-cli.md and update README.md support claims.

The per-operation SelfSubjectAccessReview fan-out profile and its `authorization` coverage payload are **not** advertised by this story. The auth contract makes exact request/response interpretation and a supporting capability implementation advertisement prerequisites; until they exist, no result claims authorization coverage. Recording that boundary is part of the story, not a silent omission.

Excluded: new Kubernetes operations (events, conditions, pod logs, exec, copy, port-forward), any Kubernetes mutation, Helm workflows, automatic discovery activation, cluster credential acquisition or refresh, and the v2 specification-derived generation and packaging pipeline. Those belong to story:kubernetes-spec-service and to later milestones of initiative:complete-local-connectors.

## Prerequisites and current state

## Prerequisites and current state

The executable composition, the `kubernetes.token` profile, SelfSubjectReview
identity validation through a check-specific capability, connection-partitioned
cursors and the three reads through the local owner are implemented and verified
against a local TLS fixture cluster and a disposable Secret Service. Receipt:
`docs/evidence/kubernetes-local-cli-20260911/README.md`. Eight Kubernetes cases
pass, the twelve existing GitLab CLI journeys still pass, and the repository gate
with `--msrv` exits 0.

A reachable Kubernetes cluster with a namespaced read-only credential is still
required for runtime acceptance; no such sandbox evidence exists in this
repository. The fixture-backed tests above do not close this story. Custody
qualification is the same Linux Secret Service binding `docs/local-secret-service.md`
already records.

The per-operation SelfSubjectAccessReview profile and its `authorization` coverage
payload remain unadvertised and unimplemented, as the Sequence and boundaries
section states.
