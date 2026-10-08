---
title: Contracts and guarantees
sidebar_position: 1
description: What a Connectors contract promises, how an interaction flows through it, and how contract, wire and adapter versions relate.
lede: A contract is the shared meaning of an interaction (types, access, lifecycle, limits and failures), and an adapter binds that meaning to one provider.
source: contracts/README.md, contracts/service/v1alpha1/semantics.md, contracts/service/compatibility.md, ess/system.yaml
---

# Contracts and guarantees

A contract describes the shared meaning of an interaction: its types, access requirements,
lifecycle, limits and failure behaviour. An adapter binds those guarantees to a provider. Shared
contracts live in the repository's `contracts/` directory and never name a provider; native
profiles (Kubernetes selectors, LogQL, PostgreSQL's text representation) live with their adapter.

## Follow an interaction

**Discover, select access, invoke, interpret.**

1. **Discover.** The caller reads the service's descriptor: each operation's id, input and output
   schema, the contract family and native profile it implements, and the descriptor's revision.
   Discovery reports observations; an endpoint a Kubernetes adapter discovers is information, not
   a connection.
2. **Select access.** A connection binds a selected target and a credential. Selecting it is a
   deliberate act of the host or the operator, never a side effect of discovery.
3. **Invoke.** The request names the operation and the descriptor revision it was built against.
   Admission decides whether this request may proceed: the caller's credential, the revision
   and the input schema, and for a write its approval, all before anything is dispatched.
4. **Interpret.** The operation and its data profile define the result, including incomplete
   pages, truncation and provenance, or a typed refusal, or an explicitly unknown outcome.

The [service contract](../reference/contracts/service.md) states the describe and invoke wire
exactly, with its limits; [Getting started](../getting-started.md) shows each step against a
running service.

## What a refusal promises

A refusal is a code with a fixed meaning, not provider prose. `invalid_input` and `unauthorized`
mean nothing was dispatched. `unavailable` means a dependency did not answer. A write whose
answer was lost after dispatch is neither success nor refusal: it is `outcome_unknown`, carries
the attempt the host recorded, and is never sent again
([governed invocation](./governed-invocation.md)). Unknown or unavailable evidence never becomes
a positive answer.

## Versions are independent

Three things carry versions, and none implies another:

| Version | Example | Owner |
|---|---|---|
| Contract family | `datasource.records/v1alpha1`, `datasource.feed/v1alpha1` | `contracts/` |
| Service wire binding | `POST /v1/invoke`, `POST /v1alpha2/invoke` | the [compatibility contract](../reference/contracts/compatibility.md) |
| Adapter specification and native profile | `kubernetes-list`, `gitlab-merge-requests/1` | the adapter |

A broader adapter specification never implies that its runtime supports every operation it
describes. Each page in the [contract reference](../reference/contracts/index.md) states its own
support status.

## Typed and textual halves

Each contract has a textual semantics document and a typed model. The models are written in
[ESS](https://beyond10x.github.io/ess/) ([GitHub](https://github.com/beyond10x/ess)) under
`ess/` for shared contracts and `adapters/<owner>/spec/ess/` for native ones; the reference
renders them as the [ESS model](../reference/contracts/model/index.md) pages. Structural
validation establishes that the model is consistent. Behaviour, storage and cross-record
predicates remain obligations each implementation discharges and its tests hold.

## Families at a glance

| Family | What it covers | Runs today |
|---|---|---|
| Service and execution | describe and invoke, the governed v1alpha2 binding, mutations and idempotency, execution audit, delegated approval, the local clock | describe and invoke; the first v1alpha2 invoke binding with audit and attempts |
| Authentication and access | connections, profiles, acquisition, custody, capabilities, readiness evidence | in part, through the local CLI's saved connections; the full contracts are specified |
| Data reads | records, logs, series, web search, feeds | records, web search (Tavily), feeds (GitLab), logs (Loki) |
| Discovery and composition | resource discovery, mediated routes, composition | endpoint and host discovery (Kubernetes) |
| Sessions and media | bidirectional sessions, negotiated media | specified only |

The [status page](../status.md) holds each runtime claim to a named test.
