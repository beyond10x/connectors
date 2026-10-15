---
format: aep.planning-md/3
id: story:adapter-private-protocol-refusal-named
kind: story
status: draft
title: An adapter refusing its private protocol is named, not unavailable
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Observed

Against 0.51.0, with a native adapter built from that tag (Prometheus) and its entry set to
`private_protocol = "connectors-private/2"`, which that adapter does not speak:
- `connections connect` answers only `{"code":"unavailable","stage":"readiness"}`, and
  `adapters status` shows the adapter `failed`.
- The adapter writes `Unsupported` to stderr, and the host discards the child's stderr
  (`crates/connectors-host/src/local/runtime/server.rs:15`), so nothing names the cause.
- `connectors-private/1` works.

## Wanted

- An adapter refusing the private protocol its entry selects is named: a typed refusal such as
  `unsupported_private_protocol` (with the protocol asked for and, where the adapter says, the
  ones it speaks), not `unavailable`.
- The README's adapter list names the private protocol(s) each adapter supports.

## Acceptance

- A host-level test starts an adapter that refuses the configured private protocol; `connections
  connect` and `adapters status` name the refusal and the protocol, and exit non-zero.
- The README table of adapters carries a private-protocol column, held by a check against each
  adapter's declared protocols so it cannot drift.

Spec first: the refusal code goes into the CLI and host ESS domains before code.
