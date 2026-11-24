---
format: aep.planning-md/3
id: story:released-install-reads-mysql-and-loki-per-environment
kind: story
status: active
title: A released install reads mysql/<env> (a managed replica) and loki/<env>
summary: The release archive ships connectors-sql; guides for a managed MySQL replica and for Loki without a Loki token; a consumer's two checks read through both
relations:
- decomposes: epic:connectors-with-claude
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: .github/workflows/release-binaries.yml
- confidence: inferred
  path: CHANGELOG.md
- confidence: cited
  path: README.md
- confidence: cited
  path: adapters/loki/README.md
- confidence: cited
  path: adapters/loki/tests/local_runtime.rs
- confidence: cited
  path: adapters/sql/src/main.rs
- confidence: inferred
  path: adapters/sql/src/mysql.rs
- confidence: cited
  path: adapters/sql/tests/mysql/fixture.rs
- confidence: inferred
  path: adapters/sql/tests/mysql_local.rs
- confidence: inferred
  path: adapters/sql/tests/mysql_protocol.rs
- confidence: cited
  path: adapters/sql/tests/mysql_replica.rs
- confidence: cited
  path: crates/connectors-build/src/ignored.rs
- confidence: cited
  path: crates/connectors-build/src/release_binaries.rs
- confidence: cited
  path: docs/development.md
- confidence: cited
  path: docs/local-mysql-cli.md
- confidence: inferred
  path: website/docs/connectors/loki.md
- confidence: inferred
  path: website/docs/connectors/mysql.md
- confidence: cited
  path: website/docs/getting-started.md
revision: 26
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T01:43:23Z", actor: "human:timo", revision: 25, decided_on: {"recorded":{"review_outcome":7}}}
- {from: "proposed", to: "active", at: "2026-11-24T01:43:24Z", actor: "human:timo", revision: 26, decided_on: {"recorded":{"review_outcome":7}}}
---
## Request

Upstream ask A15 of the dev-operator repository (its story:upstream-asks-filed), relayed by the
host's operator session on 2026-11-24: instances `mysql/<env>` (an Aurora MySQL reader
endpoint) and `loki/<env>`, read by its fleet gate and its staleness checks through the
installed `connectors` CLI.

## What exists (read from the tree at 0.76.2 on 2026-11-24, nothing built or run)

- The `mysql` connector is served by `connectors-sql` (`docs/local-mysql-cli.md`): reads
  `schema.list`, `query.read`, four catalogue reads and `profile.read`, each in a session set
  `READ ONLY`, with TLS through `ca_file`. story:parity-mysql-reads is implemented.
- `query.read` (`adapters/sql/src/mysql.rs:540-543`) sends
  `SELECT * FROM (<query>\n) AS result (c0,…) LIMIT <limit+1>`: `limit` is 1 to 1,000, the
  query at most 8,192 characters with at most 64 parameters (`adapters/sql/src/lib.rs:365-378`),
  a trailing `;` is stripped, rows stop at `limit` and `truncated` is true. The code does not
  inspect the statement: its own `LIMIT … OFFSET …` goes into the derived table unchanged.
  The wrapper has no outer `ORDER BY` unless a keyset page is requested (`mysql.rs:517-538`).
- A statement that is a write is refused `unsupported` before it runs
  (`adapters/sql/tests/mysql_protocol.rs:251-266`). A write the server refuses is answered
  `forbidden` for the MySQL errors 1044, 1142, 1143, 1227, 1370 and 1792 (`mysql.rs:640`); any
  other error falls to its SQLSTATE class.
- The answer of `query.read` has `rows` and `truncated`; the answer of `logs.query_metric` has
  `result_type` and `series` (`adapters/loki/tests/local_runtime.rs:430-441`). Neither has a
  member `status`.
- A `mysql` instance: members `format`, `engine`, `instance`, `host`, `port`, `database`, `user`,
  `allow_plaintext`, `ca_file`, `read_profiles` (`adapters/sql/src/local.rs:25-43`); the
  executable is pinned per instance by `path`, `sha256` and `args`
  (`docs/local-mysql-cli.md:80-83`); the password is entered with `--credential-prompt`.
  The guide names `ca_file` (:45-47), a user with `SELECT` only (:152-153) and the `forbidden`
  answer to a refused write (:146-147).
- The `loki` connector is served by `connectors-loki` (`adapters/loki/README.md`): reads
  `logs.query_range`, `logs.query_metric`, `logs.labels` and two profile reads. It sends a
  bearer token and no tenant header. It also reads through Grafana's data-source proxy with a
  Grafana service-account token (story:parity-grafana-datasource-loki, implemented; guide
  `adapters/loki/README.md:180-203`, the `base_url` form at :191). `query_metric` forwards the
  query string unchanged (`adapters/loki/src/lib.rs:607-626`).
- Both are pinned instances: one configuration entry per environment gives `mysql/<env>` and
  `loki/<env>`.
- The MySQL test fixture (`adapters/sql/tests/mysql/fixture.rs`) is a scripted server on
  loopback: it runs no SQL, answers every `SET …` with OK, returns the script's one set of
  columns and rows for every prepared statement and logs the statements it receives
  (`mysql_protocol.rs:625-645`). A real server is used only by ignored tests that read
  `CONNECTORS_MYSQL_ADVERSARY_PORT`: a writable server set up by hand, with a test that writes
  to it (`adapters/sql/tests/mysql_adversary_2.rs:1-4`; classified at
  `crates/connectors-build/src/ignored.rs:127` and :432-441).

## What is missing

1. A released install has no `connectors-sql`. The release archive holds `connectors`,
   `connectors-catalog-provider`, `connectors-grafana`, `connectors-prometheus`,
   `connectors-loki` and `connectors-kubernetes` (`.github/workflows/release-binaries.yml:114`).
   `mysql/<env>` is reachable today only with a build from source. No file says why it was
   left out. `connectors-sql --version` is not answered (`adapters/sql/src/main.rs:8-9` has no
   `#[command(version)]`), and the workflow runs `<binary> --version` for every listed name
   (:133-139, :380-388).
2. The guide does not name the managed-replica case: the reader endpoint as `host`, the
   provider's CA bundle as `ca_file`, and what `READ ONLY` adds on a replica. Whether a
   read-only MySQL accepts the session statements of `connectors-sql` (`mysql.rs:29` and :34)
   is not known, nor what it answers to a write that reaches it (errors 1290 and 1836 are not
   in the `forbidden` list); no test runs against one.
3. The guide does not say what `query.read` does with a statement's own `LIMIT … OFFSET …`.
   Whether a MySQL server keeps the row order of an inner `ORDER BY … LIMIT … OFFSET …`
   through the derived table is not known here; the consumer's paging selects its pages with
   it.
4. Loki through Grafana's proxy: the test (`adapters/loki/tests/local_runtime.rs:517-560`)
   covers `logs.query_range` and `logs.labels`, not `logs.query_metric`. A Loki that accepts
   requests without any credential has no route (story:loki-credential-free-connection is a
   draft and not planned); the guide does not say so.
5. Three prose lists of the archive's programs are stale or will be: `docs/development.md:206-207`
   and `README.md:309-310` name two programs, `website/docs/getting-started.md:45-50` names six
   and says the other connectors are built from source. No check compares them with the list.

## Boundary

- An instance that reaches a production database or a production Loki is configured only after
  Timo's yes for that connection, relayed by the host's operator session. This story ends at
  a fixture, a disposable local server or a non-production instance.
- The credential of each instance is passed by standard input or a file and is never printed.
- Reads only. Neither connector gets a write, and no fix of this story adds a write path or
  takes `READ ONLY` from a session.
- The consumer's statements name its schema and stay out of this repository; tests use tables
  with neutral names.

## Design

1. `connectors-sql` answers `--version` (`#[command(version)]`, as `adapters/loki/src/main.rs:7`)
   and joins `BINARIES` in `.github/workflows/release-binaries.yml:114`. Every loop over the
   list (build, version, sign, notarize, package) and the image job take the name with no
   further edit. The test at `crates/connectors-build/src/release_binaries.rs:405-419` pins the
   names: it gets the seventh and keeps its name. `connectors-sql` selects rustls on ring and
   no native TLS (`adapters/sql/Cargo.toml:32-37`); that it builds on the four release targets
   is not known. A pull request that changes the workflow file runs its four builds and the
   image build and publishes nothing (the workflow's header, lines 18-20): that run of this
   story's pull request is the proof, and a target that fails is fixed there, not excluded.
   The header comment (lines 1-2) names the programs and gets the seventh.
2. Prose: `docs/development.md:206-207`, `README.md:309-310` and
   `website/docs/getting-started.md:45-50` name the seven programs.
   `website/docs/connectors/mysql.md:28-36` says the archive holds the program and keeps the
   build from source as the second way. A new test in `release_binaries.rs` reads the three
   list files and fails when one of them lacks a name of `BINARIES`. It matches whole names
   (a name is not followed by `-` or an alphanumeric character, so `connectors-sql` does not
   count as `connectors`), through a pure function that takes a text and the names and
   returns the missing ones.
3. Wire tests on the scripted fixture (not ignored, in `adapters/sql/tests/mysql_protocol.rs`,
   each with its own `start(Script)`) pin the last statement `connectors-sql` sends for the
   consumer's shapes. They show what is sent, not what a server answers.
4. What a server answers is shown by ignored tests in a new file
   `adapters/sql/tests/mysql_replica.rs` against a real MySQL 8 that the tester starts by hand.
   They read their own variable `CONNECTORS_MYSQL_REPLICA_PORT`, with their own entry in
   `crates/connectors-build/src/ignored.rs` (a constant beside :127, an arm beside :432-441,
   family `Live`): the server behind `CONNECTORS_MYSQL_ADVERSARY_PORT` is written to by another
   test and cannot be read-only. Setup, written at the head of the file as the adversary file
   does: create the database, a user with `SELECT` and `EXECUTE` only, four tables with
   neutral names and 250 rows in the base table with a unique `id` (each joined table holds at
   most one row per base `id`, so the joins add no row), one stored function that writes a
   row; then `SET GLOBAL super_read_only = ON`. No test of the gate starts a server
   or makes a network request. The tests are run once on the workstation; the commands, the
   server's version and the results go into this story.
5. A write that reaches the server: the test calls the stored function through `query.read`
   (`SELECT <function>()`), which the static write check lets through. Every session of
   `connectors-sql` runs `SET SESSION TRANSACTION READ ONLY` (`mysql.rs:27-29`), so the
   session's own refusal (1792, in the `forbidden` list already) may answer before the
   server's `super_read_only` does (1290 or 1836). The story records the number that
   answered. When the code `connectors-sql` answers is not `forbidden`, the number joins the
   list at `mysql.rs:640` in this story, with a unit test beside the existing mapping tests.
   What a replica itself answers to a write is then not reachable through the connector, and
   the guide says whose refusal the reader sees.
6. The row order of a page. The test asserts that the page whose statement ends in
   `LIMIT 100 OFFSET 100` holds the rows 101 to 200 of the unpaged statement as a set, and
   records whether their sequence is the statement's. A different set is a defect of the
   wrapper and is fixed in this story. A different sequence is not: the guide then says that
   a page is the right rows in no promised order and that the reader sorts a page by its key,
   and the consumer gets that sentence with the story's result.
7. One Loki test more: `logs.query_metric` through `Provider::under(PROXY)`
   (`adapters/loki/tests/local_runtime.rs:115`); the fixture routes `GET /loki/api/v1/query`
   already (:92) and gets a record of the query string it received.
8. Guides: a section "A managed MySQL replica" in `docs/local-mysql-cli.md`, a paragraph on a
   statement's own `LIMIT … OFFSET …` in its `query.read` section, in `adapters/loki/README.md`
   the sentence that a Loki without any credential is not served (a limit; no route is added),
   and in `website/docs/connectors/loki.md:27-29` the same sentence beside the proxy route.
9. `CHANGELOG.md`: one entry under the next unreleased heading (the archive holds
   `connectors-sql`; `connectors-sql --version`).

## Order and ownership

- This story alone owns `connectors-sql` in `BINARIES`, its `--version` and the seven-name
  test. story:claude-getting-started-tutorial (draft) plans the same entry and rewrites the
  same prose lists and the build-from-source lines of `website/docs/connectors/mysql.md`,
  `website/docs/connectors/loki.md` and `adapters/loki/README.md`: it `depends_on` this story,
  drops `connectors-sql` from the programs it adds and starts its prose from seven. Its text
  is stale already (it counts two programs in the archive; the tree has six).
- story:test-binary-per-package-rest (draft, no scope) plans one `tests/main.rs` per package
  and would move `mysql_local.rs`, `mysql_protocol.rs`, the new `mysql_replica.rs` and the Loki
  `local_runtime.rs`, and with them the names `ignored.rs` keys on. This story lands first and
  follows the package's present layout; the fold moves its files with the others.
- `CHANGELOG.md` is shared with every story of the next release, among them
  story:standing-grant-binds-a-pattern-or-a-value-set (active, 0.78.0). This story adds its
  own entry under the unreleased heading and touches no other part. It ships in the release
  after 0.78.0, or in 0.78.0 when it is green first.
  story:test-binary-per-package-rest takes a `depends_on` edge to this story when it gets a
  scope that names `adapters/sql` or `adapters/loki`. The tutorial's own text (it still adds
  `connectors-sql`) is changed by the tutorial's revision, not here.
- Active stories that list a file of this story's `scope`: story:entity-runtime-0331-pin
  (`CHANGELOG.md`, `docs/development.md`; released with 0.76.2) and
  story:local-invoke-admits-a-keyed-write-under-a-standing-grant (`README.md`, `CHANGELOG.md`,
  `crates/connectors-build/src/ignored.rs`; 0.77.0). Draft stories:
  story:instances-resolve-through-locked-index (a step before the build in
  `.github/workflows/release-binaries.yml`), story:definitions-pin-covers-catalog
  (`docs/development.md`, the workflows), story:mounted-custody-kind-secret-volume and
  story:main-failing-ci-journeys (`ignored.rs`). The hunks are disjoint: here the `BINARIES`
  line and the header of the workflow, `docs/development.md:206-207`, `README.md:309-310`, one
  constant and one arm in `ignored.rs`, one changelog entry. The story that merges second
  merges main first.

## Acceptance

1. Test `connectors_sql_prints_its_version` in `adapters/sql/tests/mysql_local.rs` runs
   `env!("CARGO_BIN_EXE_connectors-sql")` with `--version` and asserts exit 0 and stdout
   `connectors-sql <CARGO_PKG_VERSION>` and a newline. It fails before the change.
2. `BINARIES` in `.github/workflows/release-binaries.yml` holds `connectors-sql`. The test at
   `crates/connectors-build/src/release_binaries.rs:405` asserts the seven names in order, and
   the new test of design point 2 passes with the three lists at seven names. A unit test of
   its pure function gets a text without `connectors` and a text without `connectors-sql`
   (each still holding the six other names) and asserts that exactly that name is reported
   missing. `cargo test --locked -p connectors-build --bin connectors-build release_binaries`
   runs the three. `cargo run --locked -p connectors-build --
   release-binaries` exits 0 as before (no regression; it does not show the change). In the
   pull request the four build jobs and the image job of "Release binaries" pass.
3. After the release that ships this story (not a gate of its pull request; recorded in that
   release's scope report): `tar -tzf` of the archive of each of the four targets lists
   `connectors-sql`; `connectors-sql --version` from the Linux x86_64 archive prints the
   release's version; with a `mysql` instance pinned to that file by its `sha256`,
   `connectors --output json setup check` has the check named `artifact:<alias>` with state
   `ready`.
4. `docs/local-mysql-cli.md` has a section "A managed MySQL replica" with the native file for
   a reader endpoint, the CA bundle as `ca_file`, the grants the user needs, the answer
   acceptance 5 recorded for a write that reaches the server, named as the connector's
   read-only session's refusal when the recorded number is 1792 and as the server's
   otherwise, and the sentence that this was tested on MySQL 8 with `super_read_only` and not
   on a managed service.
   `website/docs/connectors/mysql.md` names the archive as the first way to get the program.
   Checked by reading at review; `connectors-docs generate --check` passes.
5. Ignored tests in `adapters/sql/tests/mysql_replica.rs`, run with
   `CONNECTORS_MYSQL_REPLICA_PORT=<port> cargo test --locked -p connectors-sql --test
   mysql_replica -- --ignored --nocapture`, the server in `super_read_only`. `cargo run
   --locked -p connectors-build -- ignored` lists the tests of this file as `Live` with the
   prerequisite `CONNECTORS_MYSQL_REPLICA_PORT`.
   - `a_read_only_server_accepts_the_session_and_answers_a_read`: validation succeeds,
     `query.read` of `SELECT @@global.super_read_only` answers one row holding 1, and
     `query.read` of `SELECT id FROM <base> ORDER BY id` with `limit` 5 answers 5 rows and
     `truncated` true. Should the server refuse a session statement, the statement is changed
     in this story so that the session is still `READ ONLY`, and the test passes.
   - `a_write_through_a_function_is_refused_by_a_read_only_server`: `query.read` of
     `SELECT <function>()` is an error; the test prints the MySQL error number and asserts the
     code is `forbidden` (design point 5).
6. `adapters/loki/README.md` and `website/docs/connectors/loki.md` each hold the sentence that
   a Loki that accepts requests without a credential is not served, beside the Grafana proxy
   route for a consumer who has no Loki token. Checked by reading at review.
7. Wire tests in `adapters/sql/tests/mysql_protocol.rs`, each under 5 s, asserting the last
   prepared statement in the fixture's log:
   - `a_statement_with_its_own_limit_and_offset_is_sent_inside_the_derived_table`: for the
     query `SELECT id, name FROM item ORDER BY id LIMIT 100 OFFSET 100` with input `limit` 100
     and a script of 2 columns, the statement is `SELECT * FROM (SELECT id, name FROM item
     ORDER BY id LIMIT 100 OFFSET 100\n) AS result (c0,c1) LIMIT 101`.
   - `a_count_over_a_statement_is_sent_as_a_derived_table`: for the query `SELECT COUNT(*) AS
     n FROM (SELECT id, name FROM item ORDER BY id) AS matched` with `limit` 1 and a script
     of 1 column, the statement is that query inside `SELECT * FROM (…\n) AS result (c0) LIMIT
     2`.
8. Ignored tests in the same file, run as acceptance 5 (with `--nocapture`), on the four tables (1 base with 250
   rows, 3 `LEFT JOIN`, a correlated `COUNT(*)` subquery, 2 datetimes, `ORDER BY id`):
   - `the_first_page_holds_the_first_hundred_rows`: `limit` 100 answers the ids 1 to 100 and
     `truncated` true.
   - `a_page_selected_by_limit_and_offset_holds_its_rows`: the statement ending in `LIMIT 100
     OFFSET 100` with `limit` 100 answers exactly the ids 101 to 200 as a set and `truncated`
     false; the test prints whether their sequence is ascending (design point 6).
   - `a_count_over_the_statement_answers_its_rows`: the `COUNT(*)` form with `limit` 1 answers
     one row holding 250.
   Run once; the results are in this story.
9. The `query.read` section of `docs/local-mysql-cli.md` says: a statement's own `LIMIT …
   OFFSET …` is sent inside the derived table; when the statement returns more rows than
   `limit`, the answer holds `limit` rows and `truncated` true; a statement is at most 8,192
   characters, paging text and a `COUNT(*)` wrapper included; and what acceptance 8 found
   about the sequence of a page. Checked by reading at review.
10. Test `a_metric_query_reads_through_a_grafana_proxy` beside
    `adapters/loki/tests/local_runtime.rs:517`: `logs.query_metric` with the query
    `sum(count_over_time({namespace="fixture"}[1h]))` and `time_unix_ns` asserts the request
    was `GET {PROXY}loki/api/v1/query` with `Authorization: Bearer <the fixture's token>`, that
    the decoded `query` parameter the fixture received equals that text and `time` equals the
    input, and that the answer has `result_type` `vector` and the fixture's series. The query text is sent as given, so the units s, m, h
    and d and the consumer's second selector need no test of their own.
11. The consumer's two checks run their reads against a non-production `mysql/<env>` and
    `loki/<env>` and report, per read, the exit status, for `query.read` the number of
    `.result.result.rows` and `.result.result.truncated`, and for `logs.query_metric`
    `.result.result.result_type`; exit 0 with those members present is the pass. This line is
    the consumer's, in its repository. The story closes on acceptances 1, 2 and 4 to 10;
    acceptance 3 is recorded after the release, and this line's result is named here when the
    consumer sends one.

## The consumer's reads (its answer of 2026-11-24, shapes and bounds only)

Fleet gate, per `mysql/<env>` instance (found with `connectors instances list`, connector
`mysql`):

- Q1: one `SELECT` over 4 tables (1 base, 3 `LEFT JOIN`) with one correlated scalar subquery
  (`COUNT(*)`), filters `IS NULL`, `IS NOT NULL` and `!= ''`, `ORDER BY id`; 7 columns:
  integers, a string, 2 datetimes, a count.
- Q2: one `SELECT` over 2 tables (1 `LEFT JOIN`), the same filter style, `ORDER BY id`; 4
  columns.
- Paging: the first page is the statement as written with input `limit` 100; each further page
  appends `LIMIT 100 OFFSET n*100` to the statement's text; at most 40 pages (4,000 rows) per
  statement. When page 40 is full: once `SELECT COUNT(*) AS n FROM (<statement>) AS matched`.
- Calls: `connectors --output json invoke mysql/<env> query.read --input-json
  {"query":…,"limit":100}`, one at a time, each under `timeout -k 10 150`, 3 tries 20 s apart,
  no try after the step's deadline.
- Row counts per environment are not known: no `mysql` instance has run at the consumer.

Staleness check, a handful of claims per round:

- SQL: one single-aggregate `SELECT` (one row, one column) on a named reader endpoint, `limit`
  1, the same call bounds.
- Loki: one instant metric query `sum(count_over_time({<selector>}[<window>]))` and a positive
  control `sum(count_over_time({namespace="<ns>"}[<window>]))` on the same cluster: 2 calls of
  `logs.query_metric` per claim. The window is what the claim's text holds, in s, m, h or d.
- Loki route: Grafana's data-source proxy with a Grafana service-account token. The consumer
  knows no token that Loki or its gateway accepts directly.

## Out of scope

- A Loki without any credential (story:loki-credential-free-connection).
- An install script, or a `setup check` state "installed" per connector: `setup check` reports
  that for the catalog provider only (`apps/connectors/src/local.rs:772-780`).
- `.github/workflows/macos-host.yml:91` (`PROGRAMS`, four provider programs): not compared with
  `BINARIES` today and not changed here.
- `connectors-tavily` in the archive (story:claude-getting-started-tutorial).
- A managed service's own way to refuse a write on a reader: not tested here.
- Any production instance.

## Critic round 1, 2026-11-24

review-result:mysql-loki-scope-round-1 (approve), review-result:mysql-loki-acceptance-round-1,
review-result:mysql-loki-design-round-1 and review-result:mysql-loki-parallel-safety-round-1
(needs-revision). This text is the revision: the consumer's members (`rows`, `truncated`,
`result_type`; there is no `status`), the write that reaches a read-only server, the own
server variable, the named tests and commands, the page-order branch, and the section "Order
and ownership".

## Critic round 2, 2026-11-24

review-result:mysql-loki-acceptance-round-2, review-result:mysql-loki-design-round-2 and
review-result:mysql-loki-parallel-safety-round-2: approve, no blocker. Their warnings are in
this text: the session's own `READ ONLY` may answer a write before the server does (design
point 5, acceptance 4), the first replica test asserts `@@global.super_read_only`, the joins
add no row, `--nocapture`, the whole-name match of the list test, and the named stories that
share a file.
