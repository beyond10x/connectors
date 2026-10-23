---
title: Adapters and ownership
sidebar_position: 2
description: The three ways an adapter, the internal implementation of a connector, runs.
lede: An adapter runs as a standalone service or as a child of the local owner; an ordinary HTTP API needs none, because the catalog provider serves it.
source: adapters/README.md, crates/connectors-sdk/src/lib.rs
---

# Adapters and ownership

## Three ways an adapter runs

- **Standalone service.** Kubernetes and PostgreSQL also build a service executable that answers
  describe and invoke over HTTP ([getting started](../getting-started.md)).
- **Supervised by the local CLI.** The local owner starts the adapter executable as a child
  process over a private protocol and keeps its credentials in custody
  ([local runtime](./local-runtime.md)).
- **Through the catalog provider.** An ordinary HTTP API needs no adapter code at all: one engine
  serves a reviewed selection of operations from a pinned API document
  ([catalog provider](./catalog-provider.md)).
