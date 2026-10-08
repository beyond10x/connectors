---
title: Federate adapter services
sidebar_position: 1
description: Put one federation host in front of an adapter service and invoke its operations through it.
lede: A federation host is one more service with its own credential; it prefixes each downstream operation and never holds a provider credential.
source: crates/connectors-host/src/federation.rs, crates/connectors-host/tests/service.rs, examples/federation.yaml
---

# Federate adapter services

`connectors serve` runs a federation host: one endpoint whose descriptor is the union of its
downstream services' operations, each prefixed with the downstream's name. Callers authenticate
to the host with the host's credential; the host authenticates to each downstream with a
separate credential configured for it. It does one hop and never retries a provider request.

This guide continues from [Getting started](../getting-started.md): the Kubernetes adapter is
running on `127.0.0.1:17102` from `target/try`, with its caller credential in `service.secret`.

## Configure the host

Save this as `federation.yaml` in the same directory. `downstreams` lists each adapter service
with the credential the host presents to it; `allow_plaintext` admits loopback HTTP explicitly.

```yaml title="federation.yaml"
service:
  instance: engineering-gateway
  listen: 127.0.0.1:17100
  service_credential: {kind: file, path: gateway.secret}
downstreams:
  - name: kubernetes
    endpoint: http://127.0.0.1:17102/
    credential: {kind: file, path: service.secret}
    allow_plaintext: true
```

Give the host a caller credential of its own and start it:

```sh
head -c 32 /dev/urandom | base64 > gateway.secret && chmod 600 gateway.secret
../debug/connectors serve --config federation.yaml > gateway.log 2>&1 &
```

## Describe through the host

```sh
../debug/connectors describe --endpoint http://127.0.0.1:17100/ \
  --allow-plaintext --token-file gateway.secret \
  | jq '{instance, adapter, operations: [.operations[].id]}'
```

```json
{
  "instance": "engineering-gateway",
  "adapter": "federation",
  "operations": [
    "kubernetes__resources.list",
    "kubernetes__endpoints.discover"
  ]
}
```

The prefix names the source, so two downstreams can offer the same operation id. The host does
not interpret provider data.

## Invoke through the host

```sh
../debug/connectors invoke --endpoint http://127.0.0.1:17100/ \
  --allow-plaintext --token-file gateway.secret \
  --operation kubernetes__resources.list --input pods.json
```

```json
{"code":"unavailable","message":"dependency unavailable"}
```

The adapter answered as it did when called directly, and the host passed its refusal on once
without retrying it. The adapter's own caller credential does not open the host:

```sh
../debug/connectors invoke --endpoint http://127.0.0.1:17100/ \
  --allow-plaintext --token-file service.secret \
  --operation kubernetes__resources.list --input pods.json
```

```json
{"code":"unauthorized","message":"service authentication failed"}
```

## What the host does not do

- It holds no provider credential. The Kubernetes token stays in the adapter's configuration.
- It forwards one hop and never retries a provider request.
- Its downstream clients use the built-in public roots and have no private-CA setting; a
  downstream other than loopback HTTP needs a TLS endpoint with a public certificate.
- Replacing a credential file takes effect on the next request; any other configuration change
  needs a restart.

The service contract states the exact limits and result rules: [Service: describe and
invoke](../reference/contracts/service.md).
