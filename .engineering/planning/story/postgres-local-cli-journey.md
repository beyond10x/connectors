---
format: aep.planning-md/1
id: story:postgres-local-cli-journey
kind: story
status: implemented
title: Persist and reuse an admitted PostgreSQL connection through the local CLI
relations:
- decomposes: initiative:complete-local-connectors
- informed_by: story:persistent-kubernetes-journey
- informed_by: architecture-decision-record:declarative-http-provider-runtime
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: adapters/sql
- confidence: cited
  path: apps/connectors
- confidence: cited
  path: docs
revision: 5
---
## Acceptance

From a fresh private configuration on Linux x86_64, an owner configures a PostgreSQL adapter entry, enters the database password through a protected source, saves a validated connection, lists schemas and runs a parameterized read-only query through the local CLI, restarts both the CLI and the local owner, and repeats the reads using the retained exact credential version without re-entry. A write attempt, a statement outside the read-only transaction and a query exceeding the configured bounds are each refused distinguishably from an empty result.

## Why this is a different execution family

GitLab and Kubernetes both reach their provider over HTTP, so both reuse `ScopedHttp` and the capability ports in `connectors_sdk`. PostgreSQL does not: it speaks the native database protocol through `tokio_postgres`, and `Sql::new` already takes `password: Arc<dyn Credential>` rather than an HTTP port. `architecture-decision-record:declarative-http-provider-runtime` names this directly — ordinary HTTP providers become declarative templates, while "PostgreSQL needs database protocol and read-only transaction semantics". This story is that third family, and nothing in it is expected to reuse the HTTP composition beyond the host's generic connection, custody and admission machinery.

The consequence for the credential profile is concrete. The four scheme/capability pairs the private bootstrap admits are `http_bearer`, `http_basic`, `mtls` and `session_authority`. A PostgreSQL password establishes a session rather than signing each request, so `session_authority`/`session-authority` is the pair this binding selects.

## Existing owners

`adapters/sql/src/lib.rs` owns the provider behaviour and already enforces the read-only shape: each invocation opens a transaction and sets `statement_timeout`, `lock_timeout` and a fixed `search_path`, then binds a prepared single statement. `adapters/sql/contracts` owns the native contract. The generic connection, protected entry, custody, publication and reuse semantics are the same host machinery GitLab and Kubernetes already exercise, and carry no provider vocabulary.

## Sequence and boundaries

1. Add an executable composition to `adapters/sql`, in the shape `adapters/gitlab/src/local.rs` and `adapters/kubernetes/src/local.rs` establish: an owner-only `connectors-sql-local/1` native configuration, `--local-config` and `--print-local-bootstrap` on the adapter binary, a declared credential profile and a validated `Bootstrap` with one `Effect::Read` requirement per advertised operation.
2. Add the local configuration shape to `adapters/sql/spec/adapter.json` beside the federated one and regenerate the descriptor from it. Do not edit the generated file.
3. Implement identity validation over the native protocol rather than an HTTP probe: open the connection the adapter already opens, read the session's authenticated identity, and return it as the baseline. Establish what the identity is from the pinned provider behaviour before binding it, and record what was read.
4. Bind `schema.list` and `query.read` to the local owner's dispatch with their current bounds, parameterization and provenance unchanged.
5. Write deterministic failure and restart tests against a disposable local PostgreSQL instance: wrong password, a changed database identity on repair, custody unavailable, owner restart reuse, terminal revocation, and a refused write.
6. Document the journey in a counterpart to `docs/local-kubernetes-cli.md` and update the README support claims.

Excluded: MySQL, which the initiative places in the remaining-provider phase; any write, DDL or transaction control exposed to the caller; connection pooling; and the v2 specification-derived generation pipeline for this adapter.

## Prerequisites and current state

`adapters/sql/src/main.rs` supports only the federated `--config` service mode, so the adapter is unreachable from the local CLI today. That gap is what this story closes.

A disposable local PostgreSQL instance is required for runtime acceptance. Whether one can be started on this host without Timo's hands is unestablished; if it cannot, the story stays open on that evidence the same way the Kubernetes journey stays open on cluster evidence.
