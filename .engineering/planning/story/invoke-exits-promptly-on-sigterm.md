---
format: aep.planning-md/3
id: story:invoke-exits-promptly-on-sigterm
kind: story
status: draft
title: invoke exits within 5 s of SIGTERM
relations:
- decomposes: epic:review-20261123-findings
- depends_on: story:invoke-admission-queues-under-load
revision: 1
---
## Outcome

`connectors invoke` exits within 5 s of SIGTERM, leaving no lock held.

## Observed

Consumer: dev-operator (session bf-runbook), 2026-11-23: under `timeout -k 10 120`, connectors
0.65.2 took about 2 minutes to exit after SIGTERM.

## Acceptance

- A test sends SIGTERM to an invoke blocked in admission and in a provider call; the process
  exits within 5 s, and the next invoke is admitted.
