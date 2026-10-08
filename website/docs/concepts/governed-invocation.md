---
title: Governed invocation
sidebar_position: 5
description: The v1alpha2 invoke binding, which anchors every admitted call in an execution audit and tells a caller which attempt its write produced.
lede: Under v1alpha2 an admitted invocation is audited before it is dispatched, and a write learns the attempt it produced, so a lost answer is uncertain rather than silently retried.
source: contracts/service/v1alpha2/semantics.md, contracts/service/compatibility.md, contracts/operations/v1alpha1/semantics.md, crates/connectors-host/tests/http_audit_anchor.rs, crates/connectors-conformance/tests/client_v1alpha2.rs
---

# Governed invocation

A service host whose `service` configuration names a private `state` directory
(`urn:connectors:config:v2:service`) also serves `POST /v1alpha2/invoke`, the first v1alpha2
binding. Without `state`, that route answers `unavailable` and only the `/v1/*` routes are served.
Describe stays on `GET /v1/describe`, and the request carries that descriptor's `revision`.

## What the binding adds

- **An audit anchor before dispatch.** Every admitted invocation is recorded in the host's
  execution audit before the adapter is called, and the answer carries `audit_ref` and
  `audit_status`. An anchor that cannot be written refuses the call before any adapter work.
- **The attempt a write produced.** An operation that writes gets an attempt record before its
  one dispatch, and the answer's `mutation` names it: `attempt: {instance, id}`, the
  `original_request_id`, `replayed: false` and the `classification` (`applied` on success).
  Reads carry no `mutation`.
- **Uncertainty kept.** A write whose answer is lost after dispatch is `outcome_unknown` with
  its attempt, and it is not dispatched again. A refusal before dispatch carries no attempt.

Retained audit observations are held in memory and are lost when the host restarts.

## From Rust

`connectors_client::Client::invoke_v1alpha2` selects this binding and returns the result value
with the optional `MutationObservation`; a failure keeps the `mutation` when the host recorded an
attempt. The client never falls back to `/v1/invoke`: an older service that does not serve the
route is reported as `unsupported`. `Client::invoke` and the `connectors invoke` command stay on
the unchanged `/v1/invoke` binding. Since 0.35.0 `AttemptRecord.connection_ref` is optional,
which breaks callers that read it as required.

## What is still a specification

The rest of the governed service, with verified caller context, tenant and grant admission and
[delegated approval](../reference/contracts/delegation.md), is specified and not implemented. The
[governed service contract](../reference/contracts/governed-service.md) and the
[compatibility contract](../reference/contracts/compatibility.md) say which routes and codecs
exist and which are proposed. The [mutations contract](../reference/contracts/mutations.md) owns
the attempt, idempotency and outcome-unknown rules every write follows.

The [advanced contract exercises](/docs/examples/contract-exercises) let you lose a response and
watch the attempt stay uncertain, in a fictional in-memory model.
