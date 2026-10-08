---
title: Docker
sidebar_position: 8
description: Native container log and mutation contracts and a typed model; no Docker runtime.
---

# Docker

Container operations with bounded logs and explicit effects.

Native profiles describe multiplexed log handling and mutation behaviour. Docker transport and
provider semantics belong to this adapter. Socket-peer and TLS configuration need their selected
bindings. Native contract and ESS sources exist; a Docker adapter runtime is not implemented.

## Native contract reference

- [Docker logs](./contracts/logs.md)
- [Docker mutations](./contracts/mutations.md)
- The [typed native model](./model/index.md), generated from `adapters/docker/spec/ess`
