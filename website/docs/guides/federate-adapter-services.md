---
title: Federate services
sidebar_position: 1
description: Put one federation host in front of several standalone connector services from a Rust program.
lede: A federation host is one more service with its own credential; it prefixes each downstream operation and never holds a provider credential.
source: crates/connectors-host/src/federation.rs, crates/connectors-host/src/server.rs, crates/connectors-host/tests/service.rs, examples/federation.yaml
---

# Federate services

A federation host is one endpoint whose descriptor is the union of its downstream services'
operations, each prefixed with the downstream's name. Callers authenticate to the host with the
host's credential; the host authenticates to each downstream with a separate credential
configured for it. It does one hop and never retries a provider request.

The host is the library `connectors_host::federation`, which a Rust program embeds. No command of
the `connectors` binary serves it: `connectors serve` was removed in 0.60.0.

## Configure the host

`FederationConfig` holds the host's own `service` section, the same as a standalone service's, and
one `downstreams` entry per service with the credential the host presents to it.
`allow_plaintext` admits loopback HTTP explicitly. In the shape the type deserializes, for a
Kubernetes service on `127.0.0.1:7102` and a SQL service on `127.0.0.1:7103`:

```yaml title="examples/federation.yaml"
service:
  instance: engineering-gateway
  listen: 127.0.0.1:7100
  service_credential: {kind: file, path: gateway.secret}
downstreams:
  - name: kubernetes
    endpoint: http://127.0.0.1:7102/
    credential: {kind: file, path: service.secret}
    allow_plaintext: true
  - name: sql
    endpoint: http://127.0.0.1:7103/
    credential: {kind: file, path: service.secret}
    allow_plaintext: true
```

## Serve it

`Federation::connect(&config)` describes every downstream once, with that downstream's
credential, and builds the host's descriptor. It refuses a configuration with no downstream or
more than 32, a route name that is invalid, repeated or holds `__`, and a downstream that is
itself a federation host or this host: the profile supports one hop. The `Federation` it returns
implements the internal adapter interface, so
`connectors_host::server::serve(config.service.clone(), Arc::new(federation))` serves it on the
configured listener as a standalone service is served.

A caller describes and invokes the host as it would one service
([Getting started](../getting-started.md) shows both requests). The operation `resources.list` of
the `kubernetes` downstream is `kubernetes__resources.list`, so two downstreams can offer the same
operation id. A downstream's refusal reaches the caller once, as the downstream answered it.
When a downstream answers `stale_description`, the host describes it again and passes the refusal
on; it never replays the request.

## What the host does not do

- It holds no provider credential. The Kubernetes token stays in the Kubernetes service's
  configuration.
- It forwards one hop and never retries a provider request.
- It does not interpret provider data.
- Its downstream clients use the built-in public roots and have no private-CA setting; a
  downstream other than loopback HTTP needs a TLS endpoint with a public certificate.
- Replacing a credential file takes effect on the next request; any other configuration change
  needs a restart.

`crates/connectors-host/tests/service.rs` holds these rules
(`federation_preserves_result_and_reports_downstream_failure`,
`federation_refreshes_atomically_without_replaying_and_rotates_credentials`). The service
contract states the exact limits and result rules: [Service: describe and
invoke](../reference/contracts/service.md).
