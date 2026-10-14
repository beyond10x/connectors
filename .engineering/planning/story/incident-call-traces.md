---
format: aep.planning-md/3
id: story:incident-call-traces
kind: story
status: implemented
title: Follow a call's traces by its identifier across sources
relations:
- decomposes: epic:incident-collection-workflow
- depends_on: story:incident-call-lookup
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: inferred
  path: README.md
- confidence: cited
  path: adapters/loki
- confidence: inferred
  path: adapters/loki/README.md
- confidence: inferred
  path: adapters/loki/contracts/logs/v1alpha1/semantics.md
- confidence: inferred
  path: adapters/loki/design.md
- confidence: inferred
  path: adapters/loki/generated/descriptor.json
- confidence: inferred
  path: adapters/loki/spec/adapter.json
- confidence: inferred
  path: adapters/loki/spec/ess/domains/reads.yaml
- confidence: inferred
  path: adapters/loki/src/lib.rs
- confidence: cited
  path: adapters/loki/src/local.rs
- confidence: inferred
  path: adapters/loki/tests/ess_model.rs
- confidence: inferred
  path: adapters/loki/tests/fixtures
- confidence: inferred
  path: adapters/loki/tests/local_runtime.rs
- confidence: inferred
  path: crates/connectors-docs/src/status.rs
- confidence: inferred
  path: website/data/status.json
- confidence: inferred
  path: website/docs/reference/adapters/loki/contracts/logs.md
- confidence: inferred
  path: website/docs/reference/adapters/loki/index.md
- confidence: inferred
  path: website/docs/reference/adapters/loki/model/domains/connectors_loki-reads.md
- confidence: inferred
  path: website/docs/status.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T05:12:49Z", actor: "human:timo", revision: 5}
- {from: "proposed", to: "active", at: "2026-10-14T05:12:49Z", actor: "human:timo", revision: 6}
- {from: "active", to: "implemented", at: "2026-10-14T22:06:05Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A read operation follows a call's traces by its call identifier: the log lines every source that
handled the call wrote, in time order, each naming its source, within a bounded window. It is
callable through `connectors operations describe` and `invoke` with no mutation profile.

## Source

LogQL queries over the configured Loki connections (`story:parity-loki-query`), selected by the
call identifier and the window the call lookup (`story:incident-call-lookup`) answered. A tracing
backend is a later source if one is configured on the host.

## Decided before design

- Its own read operation on the Loki adapter, one connection per invocation; merging traces across
  connections is the workflow's time-ordered join.
- The query is composed from a configured trace profile (stream selector, the label or line field
  carrying the call identifier, the label naming the source), written by the operator in the host's
  local configuration; the caller passes the call identifier, the window and a limit only. The call
  identifier is escaped as a LogQL string literal. The Loki logs contract gains a profile section
  that states why this operator-authored composition is not the injected filter its section 4.2
  forbids, and that a profile can never weaken a configured scope.
- The repository holds no platform data. The call table's name and columns, the SQL text, the CDR
  line's fields, the label or field carrying the call identifier and the masking rules are the
  operator's, written in the host's local configuration as named read profiles; the repository
  holds the generic profile shape, its validation and synthetic fixtures only.

## Acceptance

- A fixture-backed test answers the lines of one call from two sources in time order, bounded by
  the window, and none of another call's.
- The output schema is published in `operations describe`.
