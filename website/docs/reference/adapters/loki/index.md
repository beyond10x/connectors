---
title: Loki
sidebar_position: 6
description: Bounded LogQL range, metric and label reads through a saved bearer connection; on main, not yet in a release.
---

# Loki

Bounded LogQL reads with explicit stream and cursor semantics.

:::note[Unreleased]

The Loki runtime is on `main` and is not yet in a tagged release.

:::

The native log profile preserves Loki query semantics, stream identity and declared time, count
and byte bounds. It specializes the shared [log datasource contract](../../contracts/data/logs.md).

## Current capabilities

Loki runs as the native executable `connectors-loki` under the local CLI, with three reads, each
one GET through a saved connection:

| Operation | What it reads |
|---|---|
| `logs.query_range` | a LogQL log query over at most 24 hours, unpaged, at most 1,000 lines |
| `logs.query_metric` | a LogQL metric query, instant or with a step |
| `logs.labels` | label names, or one label's values |

A connection uses the bearer profile `loki.bearer`. Connect, repair and `connections revalidate`
prove the token with `GET /loki/api/v1/labels`; Loki names no account, so the identity is the
configured connection. The configuration is HTTPS-only, with an optional private CA file.

## Limits

- No `X-Scope-OrgID` tenant header is sent, so every query reads the whole tenant the token
  reaches.
- A Loki without authentication cannot be connected, because the local host admits no
  credential-less profile. Other authentication profiles and tenant selection remain proposals.

The operator guide is `adapters/loki/README.md` in the repository.

## Native contract reference

- [Loki LogQL](./contracts/logs.md), whose §11 states the reads and the connection
- The [typed native model](./model/index.md), generated from `adapters/loki/spec/ess`
