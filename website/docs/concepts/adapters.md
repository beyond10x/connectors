---
title: Adapters and ownership
sidebar_position: 2
description: What an adapter owns, what stays shared, and why an adapter can leave this repository without taking shared semantics with it.
lede: Each adapter owns its provider's behaviour, contracts, model, fixtures and code in one directory, and depends on shared contracts at an exact version.
source: adapters/README.md, crates/connectors-sdk/src/lib.rs, crates/connectors-build/src/gate.rs
---

# Adapters and ownership

An adapter binds shared contracts to one provider. Everything provider-specific lives in its
directory, `adapters/<owner>/`: the native contracts and profiles, the authored ESS model, the
pinned upstream sources and their licences, generated projections, fixtures, the design notes and
the implementation. That directory is the extraction boundary. An adapter can move to its own
repository by taking the directory and replacing local paths with an exact reviewed release of
the shared contracts and SDK; no shared semantics move with it.

## What is shared and what is not

| Shared, in `contracts/` and `ess/` | Native, in `adapters/<owner>/` |
|---|---|
| The describe and invoke wire, error codes and limits | Query languages: LogQL, PromQL, SQL |
| Result envelopes: pages, cursors, provenance, completeness | Selectors and what a cursor means |
| Connections, credential custody, readiness evidence | Provider authentication profiles and their probes |
| Mutation attempts, idempotency, approval binding | Which provider call is a write and how to guard it |

Sharing a result envelope does not make native query or continuation semantics shared. A native
parser, selector or provider-specific limit changes in its adapter, without a change to the
shared contracts unless a shared guarantee changes.

## Boundaries the gate holds

The repository gate checks the boundaries rather than trusting them:

- Each adapter library built with `--no-default-features` depends on no host, client or sibling
  adapter.
- The generic `connectors` CLI depends on no provider adapter.
- Each adapter's ESS model compiles on its own; the shared model imports none of them.

A new provider supplies an adapter implementation and its configuration. The shared client and
host need no provider switch.

## Three ways an adapter runs

- **Standalone service.** Kubernetes and PostgreSQL also build a service executable that answers
  describe and invoke over HTTP ([getting started](../getting-started.md)).
- **Supervised by the local CLI.** The local owner starts the adapter executable as a child
  process over a private protocol and keeps its credentials in custody
  ([local runtime](./local-runtime.md)).
- **Through the catalog provider.** An ordinary HTTP API needs no adapter code at all: one engine
  serves a reviewed selection of operations from a pinned API document
  ([catalog provider](./catalog-provider.md)).

## Composition is explicit

A parent adapter may discover what a child adapter could use. Grafana can report a datasource and
Kubernetes can report a database endpoint, but a discovery result grants no access. A host binds
the child's target, credential and authority deliberately, and the child consumes shared mediated
capabilities without knowing its concrete parent. Grafana, Prometheus, Alertmanager and the media
adapters are specified this way, and no such mediated binding runs yet: Loki and Prometheus reach
Grafana only through a configured proxy path. [Adapters](../reference/adapters/index.md) lists
which ones run.
