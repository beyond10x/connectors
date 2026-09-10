---
title: Run your first adapter
slug: /introduction/getting-started
sidebar_position: 3
---

# Run your first adapter

The current services run from the local source checkout. You need Rust 1.88.0 or later and an explicitly configured provider endpoint. No public package distribution is configured yet.

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

## Check the local approval clock

For an existing local CLI configuration with adapter alias `forge`, the optional
clock check verifies an explicitly selected authenticated time source:

```sh
connectors approvals clock-check --adapter forge --output json
```

The private local TOML configuration needs a top-level `approval_clock` table.
The operator must admit the source's UTC assertions and a bound on the local
Linux timer's frequency error. This example uses the roughtime.se address and
key checked on September 10, 2026; recheck its current identity before use.
The 10,000 ppm value is an explicit 1% assumption, not a calibration or default.

```toml
[approval_clock]
format = "roughtime-clock/1"
address = "192.36.143.134:2002"
public_key = "S3AzfZJ5CjSdkJ21ZJGbxqdYP/SoE8fXKY0+aicsehI="
max_rate_error_ppm = 10000
```

Success returns the adapter alias and an observation containing
`configuration_sha256`, `lower_unix_ms` and `upper_unix_ms`, at most four seconds
apart. The command starts no owner or adapter and reads no credential or metadata.
An unavailable source, wrong key or uncertain time bound refuses safely.
The output is historical observation, not reusable approval evidence. Approval
issuance and provider writes remain pending. Ordinary reads need no approval
clock. See the [clock contract](/contracts/clock) for the trust assumptions,
process lifetime, suspend detection and conservative UTC-midnight refusal window.
