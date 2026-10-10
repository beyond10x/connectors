---
format: aep.planning-md/3
id: story:parity-kubernetes-exec
kind: story
status: active
title: Kubernetes pod exec as a mutation
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-10T14:23:54Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-10T14:23:55Z", actor: "human:timo", revision: 3}
---
## Outcome

`kubernetes.pod.exec` through the native Kubernetes adapter: a command run in a container of a pod in a configured namespace, as a mutation. The exec part of parity unit U19 of `docs/fluxplane-plugin-parity.md`; port-forward stays in story:parity-kubernetes-exec-portforward.

## Operations

`kubernetes.pod.exec`: 9 calls since 2026-09-09.

## Rules

The execution-family rules of decision-blocker:helm-execution-family apply: the invocation needs an approval the record names, produces an attempt, carries the command as an explicit argument vector (no shell is added), is bounded in output bytes and in time, and reports an unknown outcome as unknown. A namespace outside the configured set is refused by name.

## Acceptance

- Spec first: the operation, its input (namespace, pod, container, argument vector, deadline, output bound) and its result (exit status, stdout and stderr, truncation flags) are modelled in the Kubernetes adapter specification and its mutation contract, validated with the newest `ess`, and generated before implementation.
- Against a recorded fixture of the exec subresource: a successful command returns its exit status and bounded output; a non-zero exit is a completed attempt with that status; output over the bound is truncated and flagged; an invocation without approval is refused; a lost stream is an unknown outcome.
- The parity page row moves to covered, with the Connectors operation named.
