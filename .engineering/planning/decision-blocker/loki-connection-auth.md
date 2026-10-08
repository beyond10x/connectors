---
format: aep.planning-md/3
id: decision-blocker:loki-connection-auth
kind: decision-blocker
status: cleared
title: Nobody has decided how the local host serves a Loki connection
relations:
- blocks: story:parity-loki-query
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-08T19:36:57Z", actor: "human:timo", revision: 3}
---
## Question

How does the local host serve a Loki connection? `story:parity-loki-query` has a tested library
on branch `unit/loki-query-20261008d` (28d72d124: ESS additions, contract section 11, crate
`connectors-loki`, 21 tests), but no operation is reachable through `operations invoke`:

- `loki.anonymous`: the local host accepts only a profile with at least one entry field and a
  bearer, basic, mTLS or session scheme (`crates/connectors-host/src/local/runtime.rs:589-596`).
- `loki.bearer`: `adapters/loki/design.md` (Authentication) requires a reviewed
  identity-validation mechanism for the deployment before the profile is advertised.

## Options

| option | what | cost |
|---|---|---|
| A | the host admits a credential-free profile (`anonymous`), modelled in the shared auth contracts first; identity is the configured connection, as Runpod's `configuration` identity source | a host and contract change; anonymous Loki is then reachable |
| B | `loki.bearer` with the `configuration` identity source (the subject is the configured connection, the probe a bounded label read) | only bearer deployments; the design's identity rule is answered by the configured connection |
| C | both A and B | the larger change |

Recommended: B first. It reuses the identity source 0.36.0 shipped for Runpod and needs no host
change; A follows when an anonymous deployment is in use.

## Decision

Decided 2026-10-08: option B.

- Loki connects through the admitted `http_bearer` scheme.
- Its adapter answers the configured identity probe: `GET /loki/api/v1/labels` must return 200;
  the subject is the configuration's instance id.
- No host admission change.

Option A (a credential-free scheme, which would widen host admission at
`crates/connectors-host/src/local/runtime.rs:589-596`) is drafted as
`story:loki-credential-free-connection`, not planned.
