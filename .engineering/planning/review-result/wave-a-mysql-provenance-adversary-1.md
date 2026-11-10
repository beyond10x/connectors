---
format: aep.planning-md/3
id: review-result:wave-a-mysql-provenance-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on mysql-query-read-provenance (wave A)
relations:
- reviews: story:mysql-query-read-provenance
revision: 1
---
```
unit: story:mysql-query-read-provenance (da41df915, plus one uncommitted test file)
verdict: red
cases: executed 15→17, red 1
origin: introduced 1, pre-existing 2, undecided 0
wrote-outside-worktree: <scratch>/units/mysql-query-read-provenance/scratch/{adv-case.log,suite.log,docs-check.log}
needs-coordinator: yes
```

The code, the ESS model and the contract agree; the website page still contradicts the unit's contract paragraph.

1. `git diff --stat` empty; one new untracked test file `adapters/sql/tests/query_provenance_adversary.rs`.
2. Cases, run alone first (`cargo test --locked -p connectors-sql --test query_provenance_adversary`, exit 101, `1 passed; 1 failed`):
   - `website_page_does_not_claim_a_mysql_connection_reads_only_its_database`: RED on `website/docs/reference/adapters/sql/index.md:35` ("A MySQL connection reads only its configured database").
   - `other_database_statement_reaches_the_server_as_written`: green; `SELECT * FROM other.t` reaches the fixture unchanged, login database `fixture`, provenance `fixture`.
3. Suite: `ess_model` 15 passed; the adversary target 1 passed 1 failed (exit 101). `connectors-docs generate --check`: current.
4. Findings:
   - `website/docs/reference/adapters/sql/index.md:35`: CONFIRMED, pre-existing, warning. The false claim the story exists to correct. Fix: limit it to catalogue reads and `schema.list`; add that `query.read` follows grants.
   - `docs/local-mysql-cli.md:152-155`, `:200-202`: CONFIRMED, pre-existing, warning. The operator guide never says grants are the database boundary for `query.read` and tells readers to configure another connection to read another database.
   - `adapters/sql/contracts/reads/v1alpha1/semantics.md:113` and `reads.yaml:106`: NEEDS-CHANGE, introduced, note. "On PostgreSQL a connection cannot leave its database" is false with `dblink` or `postgres_fdw` installed and granted.
   - Story Outcome vs Acceptance: CONFIRMED, pre-existing, note. The Acceptance admits the documented option; the Outcome and title (names the database it read, or refuses) are not met by it. The coordinator restates the Outcome.
5. Could not break: a mutant provenance turns the unit's case red; model, contract and code (`lib.rs:575`, `mysql.rs:610`) agree; generated docs current; descriptor and MCP text make no provenance claim.

```findings
[{"file":"website/docs/reference/adapters/sql/index.md","line":35,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"The page says a MySQL connection reads only its configured database, while the unit's contract says query.read may read another database the user is granted; adapters/sql/tests/query_provenance_adversary.rs fails on it."},
 {"file":"docs/local-mysql-cli.md","line":200,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"The operator guide never says grants are the database boundary for query.read, and tells readers to configure another connection to read another database."},
 {"file":"adapters/sql/contracts/reads/v1alpha1/semantics.md","line":113,"category":"judgement","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"'On PostgreSQL a connection cannot leave its database' is false with dblink or postgres_fdw installed and granted; the same sentence is at reads.yaml:106."},
 {"file":".engineering story:mysql-query-read-provenance","category":"acceptance","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"The Acceptance allows the documented option but the story's Outcome and title (names the database it read, or refuses) are not met by it; the coordinator should restate the Outcome."}]
```
