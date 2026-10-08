---
format: aep.planning-md/3
id: decision-blocker:approve-wave-20261008d
kind: decision-blocker
status: open
title: Nobody has approved wave 20261008d (Entity Runtime 0.30.3, Swagger 2.0 projection, Loki)
revision: 1
---
## Question

Approve wave 20261008d: story:entity-runtime-0303-pin, story:catalog-swagger2-projection and story:parity-loki-query, delivered on one integration branch with one pull request and released when it merges.

## Options

| option | what | cost |
|---|---|---|
| A | the three units above | the pin and Loki both touch Cargo.toml and Cargo.lock; the pin merges first |
| B | the pin and Loki only | Slack (710 calls) stays blocked on the Swagger 2.0 projection one more wave |
| C | the pin alone | no parity progress this wave |

Recommended: A. Slack (710 calls) needs the projection before story:catalog-slack-reads; Loki (334 calls) has a validated ESS model; Grafana (394) waits on Loki.
