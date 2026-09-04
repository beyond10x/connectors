---
format: aep.planning-md/1
id: story:connectors-ess-domain
kind: story
status: active
title: The connectors domains get a typed home
relations:
- derived_from: epic:cli-surface
- depends_on: story:ess-clap-target
scope:
- confidence: cited
  path: ess/system/components.yaml
- confidence: cited
  path: ess/system/domains
- confidence: cited
  path: ess/system/system.yaml
revision: 6
---
# Story: the connectors domains get a typed home

## Defect

This repository has 44 crates, a 450-line domain model and no ESS specification —
`find -name system.yaml` returns six across `beyond10x` and none here. The nouns exist twice
instead: as prose in `docs/design/01-domain-model.md`, and as Rust enums in `crates/protocol/src/`.
Nothing checks that the two agree, and nothing can project either.

## The states are citable, not invented

| entity | states | source |
|---|---|---|
| `Connection` | Created, Authorized, Callable, Degraded, Revoked | `crates/protocol/src/connection.rs:112-117` |
| `Channel` | Starting, Connected, Reconnecting, Stopped | `connection.rs:147-151` |
| `DiscoveryObservation` | Observed, Unsupported, Materialized, Withdrawn | `connection.rs:212-216` |
| `ConnectionCandidate` | Detected, Activated | `connection.rs:221-223` |
| `ConnectSession` | Pending, Completed, Expired, Failed | `connection.rs:259-263` |
| `Session` (operation) | Establishing, Established, Terminating, Terminated, OutcomeUnknown | `operation.rs:194-199` |
| `SipDial` | read at model time | `sip.rs:122` |
| `Provider`, `Operation`, `Catalog`, `Integration`, `Credential`, `Grant`, `Invocation`, `Proxy`, `Event`, `Webhook`, `Delivery`, `Subscription`, `Audit` | no state enum, no drawn lifecycle | `docs/design/01-domain-model.md:109-411` |

## The constraint that sizes this

ESS refuses a transition no command outcome causes (`missing_causation`), so a list of states is not
a lifecycle. Only `Connection` has named transitions anywhere:
`created ──authorize──▶ authorized ──verify──▶ callable`, plus `reauthorize` and `revoke`
(`docs/design/01-domain-model.md:198-201`).

So, per entity:

- **Transitions stated** → model them, cite the line.
- **States but no transitions** → read the transitions from the code that performs the state change,
  or give the entity one state and an `UNMAPPED:` marker naming what would settle it.
- **Neither** → a single-state lifecycle, which is what the specification can honestly say today.

No guessed `owns` or `references`, no guessed cardinality. Every one of those is an `UNMAPPED:`
marker and is named again in the report, because one-to-many and one-to-one project different
schemas and imply different stories.

## Shape

`ess/system/system.yaml`, `ess/system/components.yaml`, `ess/system/domains/*.yaml`, laid out like
`devcenter/ess/system/`. `ess validate` after each entity, not at the end.

`connectors.target.Target` is drafted here, because `story:explicit-target-never-implicit`
introduces the noun and guardrail 7 of the `aep-planning:planning` skill says a story introducing a
noun models it first.

## Acceptance

- `ess validate --path ess/system` exits 0.
- Every entity in the table above is present, or its absence is stated with a reason.
- Every state in the specification cites a `path:line` in `crates/protocol/src/` or
  `docs/design/01-domain-model.md`.
- Every `UNMAPPED:` marker names what would settle it, and the count is reported.

## Depends on

`story:ess-clap-target` — the specification is written against the ESS version that has the
command-line construct, and validating it needs that binary installed.
