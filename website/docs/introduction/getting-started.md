---
title: Run your first adapter
slug: /introduction/getting-started
sidebar_position: 3
---

# Run your first adapter

The v0.2.0 services run from a source checkout with Rust 1.88.0 or later and an
explicitly configured provider endpoint. On Linux x86_64, start with the
[saved-credential GitLab CLI](/adapters/gitlab#use-saved-credentials).
The steps below cover the compatible standalone service interface, also used by
Kubernetes and PostgreSQL. Binary/package distribution is not configured.

## Build

From the repository root:

```sh
cargo build --workspace --locked
```

Ordinary builds read checked-in generated Rust and do not require ESS. Cargo may download dependencies on the first build.

## Configure access

Choose `examples/gitlab.yaml`, `examples/kubernetes.yaml` or `examples/sql.yaml` as your starting configuration. Set a unique instance and listener, the provider endpoint and permitted resources, and private credential references.

Credentials belong in private regular files owned by the service user with mode `0600`, or supported environment references. Caller credentials for the adapter service are separate from credentials used against its provider. Do not put either credential value into operation input.

Config changes require a service restart. Replacing a credential file is observed on subsequent requests; changing a parent shell’s environment does not change a running process.

## Start and describe

For GitLab, after preparing a configuration file named `gitlab.yaml`:

```sh
target/debug/connectors-gitlab --config gitlab.yaml
```

From another terminal, with the matching service credential in `service.secret`:

```sh
target/debug/connectors describe \
  --endpoint http://127.0.0.1:7101/ \
  --allow-plaintext --token-file service.secret
```

The endpoint and credential must match your configuration. Plaintext is explicitly enabled for loopback development. A remote placement needs the intended TLS boundary and reachability.

## Invoke a selected operation

Prepare the project input using the repository’s `examples/requests/gitlab-project.json`, matching an allowed project:

```sh
target/debug/connectors invoke \
  --endpoint http://127.0.0.1:7101/ \
  --allow-plaintext --token-file service.secret \
  --operation project.get --input examples/requests/gitlab-project.json
```

The client discovers the current descriptor before invoking. Through the example federation host the operation is `gitlab__project.get`; the prefix distinguishes its source.

Read the [service contract](/contracts/service) for exact limits and result semantics. Choose an [adapter](/adapters) for provider-specific boundaries. Local reproduction and full-gate instructions remain in the source checkout’s development guides.
