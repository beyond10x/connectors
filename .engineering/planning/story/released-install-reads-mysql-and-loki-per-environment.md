---
format: aep.planning-md/3
id: story:released-install-reads-mysql-and-loki-per-environment
kind: story
status: draft
title: A released install reads mysql/<env> (a managed replica) and loki/<env>
summary: The release archive ships connectors-sql; guides for a managed MySQL replica and for Loki without a Loki token; a consumer's two checks read through both
revision: 1
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
4. Which reads the two checks need (the SQL statements, the LogQL queries, their bounds and
   deadlines) is not stated in the ask.

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
