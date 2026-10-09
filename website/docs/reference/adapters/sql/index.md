---
title: SQL (PostgreSQL and MySQL)
sidebar_position: 4
description: Bounded, read-only PostgreSQL and MySQL reads and catalogue reads with explicit deadlines and cancellation.
---

# SQL (PostgreSQL and MySQL)

Read relational data and its catalogue with explicit bounds, on PostgreSQL or MySQL.

## Current capabilities

The SQL adapter serves PostgreSQL and MySQL, chosen per connection by the `engine` member of its
configuration; a configuration without one is PostgreSQL. Both engines implement the same six
reads:

| Operation | Reads |
|---|---|
| `query.read` | one bounded, parameterized, read-only statement |
| `schema.list` | the column metadata of one schema |
| `database.list` | the names of the databases the configured role or user may use |
| `table.list` | the tables and views of one schema, with the engine's row estimate |
| `table.describe` | one table's columns with nullability, default, primary key and foreign keys |
| `index.list` | the indexes of one table, or of a schema, one row per key column |

The four catalogue reads each run one fixed statement with the schema and table bound as
parameters, never written into the statement. Every request uses a fresh connection and a
read-only transaction (PostgreSQL) or a `READ ONLY` session (MySQL), with fixed statement and
lock timeouts and an execution deadline. A deadline or a dropped invocation sends PostgreSQL's
cancel request, or `KILL QUERY` on MySQL, and closes the connection. Under the local CLI the
session itself is the credential check, and the saved identity is the role or user and the
database.

Results preserve column names, native type names and values as text or JSON null, under the
`postgresql-native-text` or `mysql-native-text` profile. A MySQL connection reads only its
configured database: the table and index reads, like `schema.list`, refuse any other schema as
`invalid_input`. `table.describe` of a table that does not exist is `not_found`, and on MySQL
so is `table.describe` or `index.list` of a table name MySQL cannot hold. The catalogue reads
hide what the role or user cannot see as the engine's `information_schema` does; on PostgreSQL
that includes foreign keys into schemas, tables or columns the role cannot see and indexes
built on hidden columns (a whole-row reference counts as every column). SQLSTATE `0A000` returns `unsupported`.

Six real PostgreSQL 17.6 acceptance cases passed on 2026-10-02, covering saved-credential
restart, joins and UTC boundaries, result bounds, read-only refusals, native invocation-drop
cancellation and local lifecycle controls. The plaintext loopback fixture does not establish TLS
or a universal remote cancellation deadline. MySQL and the catalogue reads are verified against
scripted wire fixtures; the catalogue reads' visibility and name rules were also run against
disposable PostgreSQL 17 and MySQL 8.0 servers on 2026-10-09.

## Access and limits

Configure a restricted database role or user. PostgreSQL queries begin with
`search_path = public, pg_catalog`; qualify relations in other schemas. Read-only transactions
and sessions and deadlines are explicit boundaries, not a SQL sandbox for unrestricted
credentials. The repository's
[PostgreSQL CLI guide](https://github.com/beyond10x/connectors/blob/main/docs/local-postgres-cli.md)
and [MySQL CLI guide](https://github.com/beyond10x/connectors/blob/main/docs/local-mysql-cli.md)
cover the local configuration, the saved password and an example of each read.

## Native contract reference

- [SQL bounded reads](./contracts/reads.md)
