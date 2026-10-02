---
format: aep.planning-md/3
id: story:kubernetes-real-read-acceptance
kind: story
status: active
title: Close real Kubernetes read and local lifecycle acceptance gaps
relations:
- decomposes: initiative:complete-local-connectors
- informed_by: story:persistent-kubernetes-journey
- depends_on: story:bridge-drop-waits-for-dispatched-batch
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: adapters/kubernetes/tests/local_runtime.rs
- confidence: cited
  path: adapters/kubernetes/tests/local_runtime/cli_journey.rs
- confidence: cited
  path: adapters/kubernetes/tests/provider.rs
- confidence: cited
  path: docs/local-kubernetes-cli.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T15:18:27Z", actor: "human:timo", revision: 6, correlation: "wave-20261002d-provider-acceptance"}
- {from: "proposed", to: "active", at: "2026-10-02T15:18:28Z", actor: "human:timo", revision: 7, correlation: "wave-20261002d-provider-acceptance"}
---
## Problem and evidence

The real saved-credential restart journey and direct/federated conformance passed
on 2026-10-02 (docs/evidence/provider-restarts-20261002). The CLI journey only
requires an EndpointSlice array, which may be empty; its invalid-token case is not
an RBAC-denial case. Those facts do not establish every existing read across
restart, paging, custody and lifecycle races.

## Acceptance

All four named real-provider conformance cases pass against a task-owned disposable
cluster using production CLI/adapter binaries and qualified custody. Exact fixture
identities, observed revisions, subprocess outcomes and cleanup are retained.
Missing credentials or a missing sandbox remain explicit unexecuted evidence.

- kubernetes_cli_reuses_each_admitted_read_after_restart: resources.list,
  endpoints.discover and explicitly enabled hosts.discover return nonempty exact
  Service/EndpointSlice/node identities and correct provenance both before and
  after owner/keyring restart, without credential re-entry or connection revision
  changes. Include admitted deployments/services and disabled-host behavior.
- kubernetes_cli_provider_rbac_denial_is_not_empty_success: a valid authenticated
  service account denied one selected resource read returns the declared provider
  refusal; distinguish invalid token, empty permitted resource list and configured
  scope refusal. This does not claim SelfSubjectAccessReview coverage.
- kubernetes_cli_continuation_preserves_scope_and_revision: real paginated reads
  preserve namespace/resource selection and revision, and reject a cursor reused
  under a changed selection or connection. Unsupported resource kinds refuse
  without a provider request.
- kubernetes_cli_repair_revoke_and_stop_preserve_authority: wrong-identity repair
  preserves the admitted credential, revocation blocks new dispatch, custody loss
  cannot silently replace it, and busy stop/restart/suppression follows the existing
  owner lifecycle without a duplicate child or replayed request.

## Scope and ownership

Cited: adapters/kubernetes/tests/local_runtime/cli_journey.rs,
adapters/kubernetes/tests/local_runtime.rs, adapters/kubernetes/tests/provider.rs,
adapters/kubernetes/contracts/reads, docs/local-kubernetes-cli.md. Initial edits
are tests and guide only. Runtime code and contracts are existing owners to read;
any reproduced product defect requires a separately scoped correction. No new
entity. Root alone writes AEP and integrates new exact ignored-runner names.

## Dependencies and exclusions

Uses the reliability batch's timeout cleanup and explicit ignored runner. Grant
node get/list only to the explicitly opted-in test identity; use only the private
fixture kubeconfig. A Pending pod is not workload readiness. This unit covers
existing selected reads, not missing SelfSubjectAccessReview binding, C12 logs,
C15 restart/rollout, C16 exec/copy/tunnels, Helm or deterministic packaging. Those
remain separate initiative obligations and decision blockers where recorded.
