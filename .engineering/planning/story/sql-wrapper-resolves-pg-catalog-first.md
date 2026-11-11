---
format: aep.planning-md/3
id: story:sql-wrapper-resolves-pg-catalog-first
kind: story
status: active
title: The PostgreSQL read wrapper calls pg_catalog functions and operators, never a public shadow
tags:
- cr-002
- from-review
relations:
- decomposes: epic:zero-open-issues-and-store-defects
- serves: vision:independent-contract-adapters
scope:
- confidence: inferred
  path: adapters/sql/src/lib.rs
- confidence: inferred
  path: adapters/sql/tests/catalogue_adversary_2.rs
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-10T09:12:01Z", actor: "human:operator", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-10T09:12:01Z", actor: "human:operator", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Defect

Code review finding CR-002 (S2): PostgreSQL adapter resolves its own wrapper through `public` first.

- Citations: `adapters/sql/src/lib.rs:92` (`SET LOCAL search_path = public, pg_catalog`), `:472` (unqualified `octet_length`), `:486`, `:498` (unqualified `>` and `+`).
- What happens: a database principal with CREATE on `public` (the default before PostgreSQL 15, and kept on upgraded clusters) creates `public.octet_length(text)` or a matching operator. Every read then calls it as the connector's role. It can defeat the per-row size guard and read what that role can read; the transaction is READ ONLY, so it cannot write.
- Status: inferred. `adapters/sql/tests/catalogue_adversary_2.rs` is the binary to extend.
- Fix: `pg_catalog.octet_length`, `OPERATOR(pg_catalog.>)`, `OPERATOR(pg_catalog.+)`.

- Review's story note: new. `adapters/sql/contracts/reads/v1alpha1/semantics.md:25` states the path order for the caller's statement, not the wrapper.

## Source

The operator-ordered code review of the repository at `23b9f1bc4` (origin/main, 0.69.0), 2026-11-10, finding CR-002. No cargo command was run by the review: "verified" means the code path was traced by reading, "inferred" means it was not.

## Acceptance

- The wrapper statement calls `pg_catalog.octet_length` and the `pg_catalog` operators, so a `public.octet_length(text)` or a matching operator created by another principal is never called; a test in `adapters/sql/tests/` is red on the base.
- If no test can make it red, that evidence is recorded and nothing is changed.
