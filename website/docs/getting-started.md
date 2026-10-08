---
title: Getting started
sidebar_position: 2
description: Build Connectors from source, start one adapter service and watch it describe itself and refuse what it must.
---

# Getting started

Build the workspace, start the Kubernetes adapter as a standalone service and ask it what it
offers. No Kubernetes cluster and no real credential are needed: the service answers its
descriptor without contacting its provider, and the invocation that would need the provider is
refused by name. Every command below was run in a checkout of the repository, starting at its
root, and the output under it is what that run printed.

## Build

You need Linux x86_64, Rust 1.91 or later and `jq`. From a checkout of
[the repository](https://github.com/beyond10x/connectors):

```sh
cargo build --locked -p connectors -p connectors-kubernetes
target/debug/connectors --version
```

```text
connectors 0.37.0
```

Ordinary builds read checked-in generated Rust and need no ESS. Cargo downloads dependencies on
the first build. Binary and package distribution are not configured.

## Start an adapter service

Work in a directory under the git-ignored `target/`, with a service credential that callers
present to the adapter and a provider credential that the adapter would present to Kubernetes.
Credential files must be regular files owned by you with mode `0600`.

```sh
mkdir -p target/try && cd target/try
head -c 32 /dev/urandom | base64 > service.secret
printf 'not-a-real-token\n' > kubernetes.secret
chmod 600 service.secret kubernetes.secret
```

Save this as `kubernetes.yaml`. It permits two resource kinds in one namespace and points at a
Kubernetes API address where nothing listens:

```yaml title="kubernetes.yaml"
service:
  instance: kubernetes-local
  listen: 127.0.0.1:17102
  service_credential: {kind: file, path: service.secret}
http:
  base_url: https://127.0.0.1:6443/
  credential: {kind: file, path: kubernetes.secret}
  credential_header: authorization
  bearer: true
adapter:
  namespaces: [engineering]
  resource_kinds: [pods, services]
  discover_hosts: false
```

Start the service in the background:

```sh
../debug/connectors-kubernetes --config kubernetes.yaml > kubernetes.log 2>&1 &
```

## Ask it what it offers

```sh
../debug/connectors describe --endpoint http://127.0.0.1:17102/ \
  --allow-plaintext --token-file service.secret \
  | jq '{instance, adapter, revision, operations: [.operations[] | {id, contract, profile}]}'
```

```json
{
  "instance": "kubernetes-local",
  "adapter": "kubernetes",
  "revision": "fc07f3a36a72f565032bdb211d2b4d2e8ee58bb1ee76b13984e19746a29df181",
  "operations": [
    {
      "id": "resources.list",
      "contract": "datasource.records/v1alpha1",
      "profile": "kubernetes-list"
    },
    {
      "id": "endpoints.discover",
      "contract": "endpoint_discovery/v1alpha1",
      "profile": "kubernetes-endpointslice"
    }
  ]
}
```

The full descriptor also carries each operation's input and output schema. The revision changes
when the descriptor does; an invocation names the revision it was built against.
`--allow-plaintext` is an explicit choice for loopback development, not a TLS bypass.

## Invoke, and read the refusals

Ask for five pods. The service admits the request and dispatches it, and Kubernetes is not there:

```sh
printf '{"namespace":"engineering","kind":"pods","limit":5}\n' > pods.json
../debug/connectors invoke --endpoint http://127.0.0.1:17102/ \
  --allow-plaintext --token-file service.secret \
  --operation resources.list --input pods.json
```

```json
{"code":"unavailable","message":"dependency unavailable"}
```

A kind outside the operation's schema is refused before anything is dispatched:

```sh
printf '{"namespace":"engineering","kind":"secrets","limit":5}\n' > secrets.json
../debug/connectors invoke --endpoint http://127.0.0.1:17102/ \
  --allow-plaintext --token-file service.secret \
  --operation resources.list --input secrets.json
```

```json
{"code":"invalid_input","message":"value does not match its declared schema"}
```

A caller without the service credential is refused before it learns anything:

```sh
head -c 32 /dev/urandom | base64 > other.secret && chmod 600 other.secret
../debug/connectors describe --endpoint http://127.0.0.1:17102/ \
  --allow-plaintext --token-file other.secret
```

```json
{"code":"unauthorized","message":"service authentication failed"}
```

Each refusal exits with status 1. Leave the service running for the next guide; it stops with
your shell's job control.

## Next

- Put a federation host in front of this service: [Federate adapter services](./guides/federate-adapter-services.md).
- Use the local CLI with saved connections: [Set up the local CLI](./guides/set-up-the-local-cli.md).
- See what each adapter can do and which ones run: [Adapters](./reference/adapters/index.md) and the [status page](./status.md).
- Follow an illustrated request from a laptop to GitLab: [Follow a request](/docs/examples/follow-a-request).
