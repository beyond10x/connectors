---
format: aep.planning-md/3
id: story:released-install-reads-mysql-and-loki-per-environment
kind: story
status: draft
title: A released install reads mysql/<env> (a managed replica) and loki/<env>
summary: The release archive ships connectors-sql; guides for a managed MySQL replica and for Loki without a Loki token; a consumer's two checks read through both
revision: 2
---
## Request

Upstream ask A15 of the dev-operator repository (its story:upstream-asks-filed), relayed by the
host's operator session on 2026-11-24: instances `mysql/<env>` (an Aurora MySQL reader
endpoint) and `loki/<env>`, read by its fleet gate and its staleness checks through the
installed `connectors` CLI.

## What exists (read from the tree at 0.76.1, nothing run for this story)

- The `mysql` connector is served by `connectors-sql` (`docs/local-mysql-cli.md`): reads
  `schema.list`, `query.read`, four catalogue reads and `profile.read`, each in a session set
  `READ ONLY`, with TLS through `ca_file`. story:parity-mysql-reads is implemented.
- The `loki` connector is served by `connectors-loki` (`adapters/loki/README.md`): reads
  `logs.query_range`, `logs.query_metric`, `logs.labels` and two profile reads. It sends a
  bearer token and no tenant header. It also reads through Grafana's data-source proxy with a
  Grafana service-account token (story:parity-grafana-datasource-loki, implemented).
- Both are pinned instances: one configuration entry per environment gives `mysql/<env>` and
  `loki/<env>`.

## What is missing

1. A released install has no `connectors-sql`. The release archive holds `connectors`,
   `connectors-catalog-provider`, `connectors-grafana`, `connectors-prometheus`,
   `connectors-loki` and `connectors-kubernetes` (`.github/workflows/release-binaries.yml:114`).
   `mysql/<env>` is reachable today only with a build from source.
2. No guide names what an Aurora reader needs: the reader endpoint as `host`, the provider's CA
   bundle as `ca_file`, a user with `SELECT` only, and what `READ ONLY` adds on a replica.
   Whether the session statements of `connectors-sql` are accepted by an Aurora reader is not
   known; no test runs against Aurora.
3. Loki: which of the two routes the consumer's deployment accepts is not known (a token at
   Loki or its gateway, or Grafana's proxy). A Loki that accepts requests without any
   credential has no route: story:loki-credential-free-connection is a draft and not planned.
4. Which reads the two checks need was not stated in the ask; the consumer named them afterwards
   (section "The consumer's reads" below).

## Boundary

- An instance that reaches a production database or a production Loki is configured only after
  Timo's yes for that connection, relayed by the host's operator session. This story ends at
  a fixture or a non-production instance.
- The credential of each instance is passed by standard input or a file and is never printed.
- Reads only. Neither connector gets a write.

## Acceptance

1. The release archive for each released target holds `connectors-sql`, and the release check
   that lists the archive's programs names it.
2. On an install made from that archive alone, `connectors setup check` reports the `mysql`
   provider as installed, and `query.read` against the repository's MySQL fixture answers
   through the installed binary.
3. `docs/local-mysql-cli.md` has a section "A managed MySQL replica" with the native file for
   a reader endpoint, the CA bundle, the grants the user needs, and the refusal a write
   statement gets on a replica.
4. A test against the MySQL fixture started read-only (`super_read_only` or the fixture's
   equivalent) shows that the session statements of `connectors-sql` are accepted and
   `query.read` answers.
5. The Loki guide names, for a consumer who has no Loki token, the Grafana proxy route with
   the exact `base_url` form, and says that a Loki without any credential is not served.
6. The consumer's two checks run their reads against a non-production `mysql/<env>` and
   `loki/<env>` and report the exit status and the answer's `result.result.status` for each;
   their statements and queries are recorded in this story.

## The consumer's reads (its answer of 2026-11-24, shapes and bounds only)

The statements name the consumer's schema and stay out of this repository; a fixture with
neutral names stands in for them.

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

What this adds to the acceptance:

7. The MySQL fixture holds tables with neutral names that Q1's and Q2's shapes run against
   (4 tables, 3 `LEFT JOIN`, a correlated `COUNT(*)` subquery, datetimes), and a test reads
   both through `query.read` with `limit` 100, then a page whose statement text ends in
   `LIMIT 100 OFFSET 100`, then the `COUNT(*)` over the statement as a derived table.
8. The guide says whether a statement that carries its own `LIMIT … OFFSET …` is admitted
   beside the input's `limit`, and what `query.read` answers when the statement returns more
   rows than `limit`.
9. A test reads `logs.query_metric` with `sum(count_over_time({…}[1h]))` through a `base_url`
   that is a Grafana data-source proxy path (fixture).
