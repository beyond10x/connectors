---
title: SQL / PostgreSQL
sidebar_position: 4
description: Bounded, read-only PostgreSQL reads with explicit deadlines and cancellation.
---

# SQL / PostgreSQL

Read relational data with explicit bounds.

## Current capabilities

The PostgreSQL adapter implements `schema.list` and `query.read`. Every request uses a fresh
connection and a read-only transaction, with fixed statement and lock timeouts and an execution
deadline. A deadline or a dropped invocation sends PostgreSQL's cancel request and drains the
connection. Under the local CLI the session itself is the credential check, and the saved
identity is the role and database.

Results preserve column names, native type names and values as text or JSON null. PostgreSQL
resolves text parameters; use explicit SQL casts where needed. SQLSTATE `0A000` returns
`unsupported`.

Six real PostgreSQL 17.6 acceptance cases passed on 2026-10-02, covering saved-credential
restart, joins and UTC boundaries, result bounds, read-only refusals, native invocation-drop
cancellation and local lifecycle controls. The plaintext loopback fixture does not establish TLS
or a universal remote cancellation deadline.

## Access and limits

Configure a restricted database role. Queries begin with `search_path = public, pg_catalog`;
qualify relations in other schemas. Read-only transactions and deadlines are explicit boundaries,
not a SQL sandbox for unrestricted credentials. The repository's
[PostgreSQL CLI guide](https://github.com/beyond10x/connectors/blob/main/docs/local-postgres-cli.md)
covers the local configuration and saved password.

## Native contract reference

- [PostgreSQL bounded reads](./contracts/reads.md)
