---
format: aep.planning-md/3
id: release-plan:connectors-v0400-mysql-reads
kind: release-plan
status: implemented
title: 'Release 0.40.0: MySQL and catalogue reads in the SQL adapter'
relations:
- serves: vision:independent-contract-adapters
- informed_by: story:parity-mysql-reads
revision: 3
transitions:
- {from: "draft", to: "active", at: "2026-10-09T22:39:34Z", actor: "human:timo", revision: 2}
- {from: "active", to: "implemented", at: "2026-10-09T22:39:34Z", actor: "human:timo", revision: 3}
---
## Outcome

Minor release 0.40.0: the SQL adapter serves MySQL beside PostgreSQL, chosen per connection by
`engine`, and both engines answer four catalogue reads (`database.list`, `table.list`,
`table.describe`, `index.list`).

Minor rather than patch: a new configuration member (`engine`), a new connection profile
(`mysql.password`), four new declared reads that change the SQL descriptor's revision, and a
public `engine` field on the library's `Config`. A configuration without `engine` is PostgreSQL
and keeps its configuration revision.

## Scope

| surface | change |
|---|---|
| `adapters/sql/spec/ess/` | the adapter's model: engine, configuration, password profile, read binding, MySQL cell rules, `connectors_sql.reads.CatalogScope` and the row types |
| `adapters/sql/` | the MySQL engine (`mysql_async` on rustls), a read-only UTC session per read, `KILL QUERY` on deadline or drop; the catalogue reads on both engines with their visibility rules |
| `adapters/sql/contracts/reads/v1alpha1/semantics.md` | the catalogue reads' semantics |
| `crates/connectors-build/src/ignored.rs` | the new live cases classified `live` |
| `docs/local-mysql-cli.md`, `CHANGELOG.md`, `Cargo.toml`, `Cargo.lock` | operator guide and version 0.40.0 |

Stories: `story:parity-mysql-reads` under `epic:fluxplane-plugin-parity` (implemented). Drafted
from the wave and not part of this release: `story:mysql-query-read-provenance`,
`story:sql-length-units`.

## Evidence

- Wave PR https://github.com/beyond10x/connectors/pull/144, head `7bfa584dc`, merged 2026-10-09
  21:34Z as `a92e7fad5`; Rust gate, Documentation validation, Planning store and Shared source
  gates passed (Rust gate run https://github.com/beyond10x/connectors/actions/runs/37991497082).
- Adversary: two passes per unit, recorded as `review-result:adversary-mysql-engine-pass-1`,
  `-pass-2`, `review-result:adversary-sql-catalogue-reads-pass-1` and `-pass-2`; every finding
  fixed before the merge.
- Ignored live cases ran against disposable PostgreSQL 17.11 and MySQL 8.0.46 servers.
- Release PR https://github.com/beyond10x/connectors/pull/145, head `ca84f6ec1`, merged 22:06Z as
  `d8b471af7`; its checks and the `main` runs on `d8b471af7` (Rust gate, Documentation validation,
  Documentation site, Planning store, Shared source gates) passed.
- Tag `v0.40.0`: annotated, tagger `b10x-bot[bot]`, peels to `d8b471af7`, an ancestor of
  `origin/main`. Release page by the bot, Latest, published 22:08:26Z:
  https://github.com/beyond10x/connectors/releases/tag/v0.40.0

## Completion boundary

There is no real-provider MySQL evidence yet: the engine is verified on a scripted wire fixture
and disposable servers. The catalogue statements have not run on a live provider. MariaDB is
not tested and SQLite is not served. `schema.list` stays column metadata only on both engines.
