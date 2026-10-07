---
format: aep.planning-md/3
id: story:parity-alertmanager-alerts
kind: story
status: draft
title: Alertmanager active alerts
relations:
- decomposes: epic:fluxplane-plugin-parity
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Alertmanager active alerts — parity unit U24 of `docs/fluxplane-plugin-parity.md`.

## Operations

`alertmanager.alerts`

2 calls since 2026-09-09 (declared and mapped undeclared names), provider `alertmanager`, planned wave W6.

## Surface

new native adapter; `adapters/alertmanager` holds a design only; the alert record is not modelled.

## Acceptance

- Spec first: what the unit adds is modelled in the ESS specification it belongs to (adapter model, or pinned OpenAPI source and selection) and validated with the newest `ess` before implementation.
- Each operation above answers through `connectors operations invoke` on a saved connection, against a recorded provider fixture, with the same capability the fluxplane operation gives (the parity page names the gap per operation).
- The parity page row of each operation moves to covered, with the Connectors operation named.
