---
format: aep.planning-md/3
id: story:incident-call-lookup
kind: story
status: implemented
title: Look up a call by its number, one typed line per leg
relations:
- decomposes: epic:incident-collection-workflow
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: inferred
  path: README.md
- confidence: inferred
  path: adapters/loki/README.md
- confidence: inferred
  path: adapters/loki/contracts/logs/v1alpha1/semantics.md
- confidence: inferred
  path: adapters/loki/design.md
- confidence: cited
  path: adapters/loki/generated/descriptor.json
- confidence: cited
  path: adapters/loki/spec/adapter.json
- confidence: cited
  path: adapters/loki/spec/ess/domains/reads.yaml
- confidence: cited
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
  path: adapters/loki/tests/protocol.rs
- confidence: inferred
  path: adapters/sql/contracts/reads/v1alpha1/semantics.md
- confidence: cited
  path: adapters/sql/generated/descriptor.json
- confidence: cited
  path: adapters/sql/spec/adapter.json
- confidence: cited
  path: adapters/sql/spec/ess/domains/reads.yaml
- confidence: cited
  path: adapters/sql/src/lib.rs
- confidence: cited
  path: adapters/sql/src/local.rs
- confidence: inferred
  path: adapters/sql/src/mysql.rs
- confidence: inferred
  path: adapters/sql/tests/ess_model.rs
- confidence: inferred
  path: adapters/sql/tests/local_runtime.rs
- confidence: inferred
  path: adapters/sql/tests/mysql/fixture.rs
- confidence: inferred
  path: adapters/sql/tests/mysql_protocol.rs
- confidence: inferred
  path: adapters/sql/tests/protocol.rs
- confidence: inferred
  path: crates/connectors-conformance/src/main.rs
- confidence: cited
  path: crates/connectors-docs/src/status.rs
- confidence: inferred
  path: docs/local-mysql-cli.md
- confidence: inferred
  path: docs/local-postgres-cli.md
- confidence: cited
  path: website/data/status.json
- confidence: inferred
  path: website/docs/reference/adapters/loki
- confidence: inferred
  path: website/docs/reference/adapters/sql
- confidence: cited
  path: website/docs/status.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-14T05:12:48Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-14T05:12:49Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-10-14T22:06:05Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1}}}
---
## Outcome

A read operation looks up a call by its number and answers one typed line per leg: direction,
caller and callee numbers (masked), start and end, the call identifier and the leg's type. It is
callable through `connectors operations describe` and `invoke` with no mutation profile, so a
workflow reads it without a model.

## Source

The SIP proxy's CDR lines in Loki, joined to the platform's call table read with SQL on a read
replica, matched on direction, numbers and a time window (see `epic:incident-collection-workflow`,
"A call by its number"). Homer SIP capture is a later source, once its credential profile exists.

What the join must handle, as an existing operator tool found:

- a transfer leg writes no call-table row, and the CDR line carries no leg type, so neither source
  alone lists every leg with its type;
- the SQL read returns at most 100 rows whatever the query's `LIMIT` says, so the operation pages;
- end-user numbers are masked by default and stay masked in the typed output.

## Decided before design

- Two typed reads, joined by the calling workflow; no composite operation. Each read is callable
  through `operations describe/invoke` without a mutation profile and returns typed output that
  carries the join key (the call identifier), so a deterministic workflow step joins them without a
  model. A composite operation would be a new runtime noun and stays out until two consumers need it.
- Each read binds to a configured connection the caller names by connection id (one Loki, one SQL);
  nothing is discovered from the host configuration implicitly.

- The repository holds no platform data. The call table's name and columns, the SQL text, the CDR
  line's fields, the label or field carrying the call identifier and the masking rules are the
  operator's, written in the host's local configuration as named read profiles; the repository
  holds the generic profile shape, its validation and synthetic fixtures only.
- SQL: a configured read profile in the SQL adapter: operator-authored parameterised SQL, typed
  output columns, a keyset cursor over a declared ordering column, and per-column masking; the
  caller passes only declared parameters. Loki: a configured line profile parses matching lines
  into declared typed fields (logfmt or a declared pattern), masking per field.
- The join key is whichever output field each profile declares as the call identifier; the
  fixtures prove a join on it, and the workflow owns the join.

## Acceptance

- A fixture-backed test answers every leg of a transferred call, with its type, from the two
  sources, across a page boundary of the SQL read.
- The output schema is published in `operations describe`, and numbers are masked.
