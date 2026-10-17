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
Runpod REST API document. The words connector, instance, connection and operation are those of
the [README vocabulary](https://github.com/beyond10x/connectors#vocabulary); the
[full Runpod guide](https://github.com/beyond10x/connectors/blob/main/docs/catalog-runpod.md) has
every outcome.

Commands use `connectors` on your `PATH` with your own configuration. Outputs were produced in
the repository's tree against a scratch configuration, whose `--config` and `--state-dir` are left
out here. A command that needs a Runpod account is shown without output.

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

```sh
connectors instances list
```

```json
{"adapters":[{"adapter":"runpod.gpu","adapter_id":"catalog","connector":"runpod","instance":"gpu","instance_id":"runpod.gpu","restart":"never","startup":"on-demand"}],"source":"configuration"}
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
connection=$(connectors --output json connections list --connector runpod --instance gpu \
  | jq -r '.result.connections[0].connection')
schema=$(connectors --output json operations describe --connector runpod --instance gpu \
  --operation pods.list | jq -r .result.schema)
revision=$(connectors --output json operations describe --connector runpod --instance gpu \
  --operation pods.list | jq -r .result.revision)
connectors operations invoke --connector runpod --instance gpu --connection "$connection" \
  --operation pods.list --schema "$schema" --revision "$revision" \
  --input-json '{"name":"worker-a","desiredStatus":"RUNNING"}'
```

## A first guarded write

Once per instance, create the signing key (`connectors approvals key-init --connector runpod
--instance gpu`) and add an `[approval_clock]` table to the configuration
([the approval clock](https://github.com/beyond10x/connectors/blob/main/docs/local-clock.md)).
Publish a policy naming the writes:

```sh
printf '{"operations":["pod.create","pod.terminate"]}\n' > policy.json
connectors approvals policy-set --connector runpod --instance gpu --input-file policy.json
```

Describe the pod in `create.json`, with a `name` unique to it:

```json
{"body": {"name": "worker-a", "imageName": "registry.example.com/team/worker:1",
          "gpuTypeIds": ["NVIDIA GeForce RTX 4090"], "gpuCount": 1,
          "containerDiskInGb": 20, "volumeInGb": 0, "ports": ["8000/http"],
          "env": {"MODE": "serve"}, "cloudType": "SECURE"}}
```

Take the schema and revision of `pod.create` from `operations describe`, then prepare, issue and
invoke:

```sh
connectors --output json approvals prepare --connector runpod --instance gpu \
  --connection "$connection" --operation pod.create \
  --schema "$schema" --revision "$revision" --input-file create.json
connectors approvals clock-check --connector runpod --instance gpu
connectors approvals issue --connector runpod --instance gpu \
  --connection "$connection" --operation pod.create \
  --schema "$schema" --revision "$revision" --input-file create.json \
  --approve-subject "$subject_sha256" --proof-output "$HOME/private/proof-worker-a.json"
connectors operations invoke --connector runpod --instance gpu --connection "$connection" \
  --operation pod.create --schema "$schema" --revision "$revision" \
  --input-file create.json --approval-file "$HOME/private/proof-worker-a.json" \
  --idempotency-key worker-a
```

A create Runpod refuses because it has no machine to place the pod on is `refused` with code
`capacity`: no pod was created, and the same workload may be placed on another GPU type, data
center or cloud type, or later. A create answered `5xx`, or lost after it was sent, is `unknown`:
the pod may exist and be billed. List by the same `name` before creating again. Terminate the same way with `pod.terminate` and
`{"podId": "<id from the create answer>"}`.

## Reconnect when the connection is Stale

A key Runpod refuses makes the connection `stale`, and an invoke answers `next_action:
repair_connection`. Publish a new key of the same account on the same connection:

```sh
connectors --output json connections list --connector runpod --instance gpu \
  | jq -c '[.result.connections[] | {connection, revision, state}]'
connectors connections reconnect --connector runpod --instance gpu \
  --expected-revision "<revision from the list>" --credential-file ~/private/runpod.json
```

Reconnect cannot tell two Runpod accounts apart: after a reconnect with another account's key,
the first account's pods answer `pod.terminate` with `404` while they keep running and billing.

## Limits

- Verified against a local HTTPS fixture only (`adapters/catalog/tests/runpod.rs`); no live
  Runpod account has been called and no pod created.
- `pod.terminate` answered `404` terminated nothing under the current key; it does not prove the
  pod is gone.
- Stop, start, reset, update, templates, volumes and every billing read but `billing.pods` are not
  offered.
