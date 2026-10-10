---
format: aep.planning-md/3
id: story:parity-kubernetes-reads
kind: story
status: active
title: Kubernetes namespaces, single objects, events and rollout history
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-10T09:55:31Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-10T09:55:32Z", actor: "human:timo", revision: 3}
---
## Outcome

Kubernetes namespaces, single objects, events and rollout history — parity unit U18 of `docs/fluxplane-plugin-parity.md`.

## Operations

`kubernetes.namespace.list`, `kubernetes.pod.show`, `kubernetes.deployment.show`, `kubernetes.deployment.history`, `kubernetes.event.list`

17 calls since 2026-09-09 (declared and mapped undeclared names), provider `kubernetes`, planned wave W3.

## Surface

native adapter change: namespaces, events, ReplicaSets, a single-object read.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.
