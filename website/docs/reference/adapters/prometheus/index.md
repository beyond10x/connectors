---
title: Prometheus
sidebar_position: 10
description: A native PromQL series contract behind the shared series family; no Prometheus runtime.
---

# Prometheus

PromQL series semantics through a shared datasource boundary.

The native profile describes instant and range queries, time and step bounds, series identity and
partial results. It keeps PromQL semantics with the provider and specializes the shared
[time series contract](../../contracts/data/series.md). A native contract and an adapter design
exist; a typed model and a runtime do not.

## Native contract reference

- [Prometheus PromQL](./contracts/series.md)
