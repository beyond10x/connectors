---
title: Runpod
sidebar_position: 13
description: List Runpod pods and their billing, and create and terminate pods under approval.
lede: One Runpod instance per account lists pods, reads their billing and creates or terminates them under approval, each write sent exactly once.
source: docs/catalog-runpod.md, adapters/catalog/providers/runpod/connector.json, adapters/catalog/providers/runpod/operations.json, adapters/catalog/tests/runpod.rs
connectors: "runpod"
---

# Runpod

The `runpod` connector of the provider Runpod is served by the catalog provider from the pinned
Runpod REST API document. Terms follow the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Runpod guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-runpod.md) has
every outcome.

[How these commands were run](../guides/first-read.md#examples).

## What it reads and writes

| Reads | Writes |
|---|---|
| `pods.list`, `billing.pods` | `pod.create` and `pod.terminate`, both unguarded: neither reads the pod first |

## Prerequisites at the provider

Create an API key in the Runpod console's settings with access to pods. The key is the only
credential; Runpod has no account read, so the connection's identity is the configured instance.
Use one instance per Runpod account.

## Enable the connector and add an instance

The connector's default endpoint is `https://rest.runpod.io/v1`:

```toml
[connectors.runpod]
enabled = true

[connectors.runpod.instances.gpu]
```

## Connect

Put `{"token": "<Runpod API key>"}` in an owner-only file, then:

```sh
connectors connections connect --connector runpod --instance gpu \
  --profile runpod.api-key --credential-file ~/private/runpod.json
```

Connecting proves the key with one `GET /v1/pods`.

## A first read

```sh
connectors invoke runpod/gpu pods.list \
  --input-json '{"name":"worker-a","desiredStatus":"RUNNING"}'
```

## A first guarded write

Set up the [signing key and clock](../guides/guarded-write.md#once-per-instance), then publish this policy with `approvals policy-set`:

```sh
printf '{"operations":["pod.create","pod.terminate"]}\n' > policy.json
```

Describe the pod in `create.json`, with a `name` unique to it:

```json
{"body": {"name": "worker-a", "imageName": "registry.example.com/team/worker:1",
          "gpuTypeIds": ["NVIDIA GeForce RTX 4090"], "gpuCount": 1,
          "containerDiskInGb": 20, "volumeInGb": 0, "ports": ["8000/http"],
          "env": {"MODE": "serve"}, "cloudType": "SECURE"}}
```

[Prepare, issue and invoke](../guides/guarded-write.md#prepare-issue-and-invoke) `pod.create` on `runpod/gpu` with `create.json`, `--idempotency-key worker-a`.

A create Runpod refuses because it has no machine to place the pod on is `refused` with code
`capacity`: no pod was created, and the same workload may be placed on another GPU type, data
center or cloud type, or later. A create answered `5xx`, or lost after it was sent, is `unknown`:
the pod may exist and be billed. List by the same `name` before creating again. Terminate the same way with `pod.terminate` and
`{"podId": "<id from the create answer>"}`.

## Reconnect when the connection is Stale

A key Runpod refuses makes the connection `stale`. Publish a new key of the same account on the same connection. [Reconnect](../guides/reconnect-a-connection.md) `runpod/gpu` with `--credential-file ~/private/runpod.json`.

Reconnect cannot tell two Runpod accounts apart: after a reconnect with another account's key,
the first account's pods answer `pod.terminate` with `404` while they keep running and billing.

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/runpod.rs`); no live
  Runpod account has been called and no pod created.
- `pod.terminate` answered `404` terminated nothing under the current key; it does not prove the
  pod is gone.
- Stop, start, reset, update, templates, volumes and every billing read but `billing.pods` are not
  offered.
