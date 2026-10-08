---
title: Grafana
sidebar_position: 9
description: Native datasource discovery and mediated-route contracts and a typed model; no Grafana runtime.
---

# Grafana

Discover datasource candidates and describe mediated access.

The native model records datasource discovery and route behaviour. A discovered datasource is
information, not authority to use its underlying provider. A composition can pair Grafana with a
separately implemented child adapter through shared capabilities; child adapters do not import
Grafana implementation details. No Grafana runtime is implemented.

## Native contract reference

- [Grafana datasource discovery](./contracts/discovery.md)
- [Grafana mediated routes](./contracts/routes.md)
- The [typed native model](./model/index.md), generated from `adapters/grafana/spec/ess`
