---
format: aep.planning-md/3
id: story:kubernetes-eks-exec-auth-and-kinds
kind: story
status: implemented
title: Kubernetes EKS exec-plugin auth, nodes, statefulsets and custom resources
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T23:35:16Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-14T23:35:16Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-16T02:05:08Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

The Kubernetes adapter reads an EKS cluster whose kubeconfig authenticates through an exec plugin,
and lists nodes, statefulsets and custom resources of a named kind.

## Today

Reported by a downstream consumer on connectors 0.50.0, 2026-10-14: an EKS kubeconfig (exec-plugin
credential, `aws eks get-token`) cannot be used, and nodes, statefulsets and CRD kinds have no read.
3 operations, 1 used every round.

## Change

Spec first in the Kubernetes adapter's ESS model and contract: an exec-plugin credential is a
declared credential source whose command, arguments and environment come from the kubeconfig the
operator selects; the adapter runs it under the rules of the execution family already decided for
external provider binaries (check `decision-blocker:helm-execution-family` and its outcome before
modelling, and stop with a decision request if exec plugins fall outside it). Then the reads: nodes,
statefulsets, and custom resources by group, version and plural, each paged.

## Acceptance

- A fixture exec plugin returning an ExecCredential authenticates a recorded API session.
- Node, statefulset and custom-resource lists answer against recorded fixtures.

## Scope

Primary: adapters/kubernetes (spec/ess/domains/reads.yaml, contracts auth+reads semantics.md, spec/adapter.json; src/lib.rs collection :722, list :345, resource_kinds :402; src/local.rs; src/kubeconfig.rs; src/auth.rs); generated descriptor.json; connectors-docs. Open: who runs the exec plugin (adapter or host; host would collide with host/CLI); whether decision-blocker:helm-execution-family admits kubeconfig-selected exec plugins (stop with decision request if not). Disjoint from cli.yaml, local.rs, registry, catalog for the reads half.
