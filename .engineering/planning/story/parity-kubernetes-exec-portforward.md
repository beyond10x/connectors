---
format: aep.planning-md/3
id: story:parity-kubernetes-exec-portforward
kind: story
status: draft
title: Kubernetes exec and port-forward
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Kubernetes exec and port-forward — parity unit U19 of `docs/fluxplane-plugin-parity.md`.

## Operations

`kubernetes.pod.exec`, `kubernetes.portforward.start`, `kubernetes.portforward.stop`

13 calls since 2026-09-09 (declared and mapped undeclared names), provider `kubernetes`, planned wave W4.

## Surface

native adapter change: streaming subresources; the execution profile is deferred (`README.md:115-116`).

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.

## Note

Execution runs a remote process: it needs the bounded process family decided for Helm (`decision-blocker:helm-execution-family`, cleared 2026-10-03) or its own decision.
