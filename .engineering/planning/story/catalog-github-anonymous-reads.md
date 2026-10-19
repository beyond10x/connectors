---
format: aep.planning-md/3
id: story:catalog-github-anonymous-reads
kind: story
status: draft
title: Public GitHub reads with no token, on a credential-less profile admitted only for public read operations
relations:
- depends_on: story:catalog-github-reads
- depends_on: architecture-decision-record:credential-less-public-reads
revision: 1
---
## Outcome

A consumer reads public GitHub repository metadata through the Connectors CLI with no token: `repos.list`, `repo.get`, `releases.list`, `release.latest`, `readme.raw` and `file.raw` of `story:catalog-github-reads` run on a credential-less profile. Every other operation, in GitHub and in every other provider, keeps the host's credential rule unchanged.

## Load it must carry

The consumer's run is `repos.list` plus `release.latest` per repository: about 55 calls every 6 hours, under GitHub's 60 requests per hour per IP for unauthenticated calls. Conditional requests keep repeat runs cheap: a 304 answer does not count toward that limit.

## Design first

The host today refuses a profile with no credential fields and admits only credential schemes (`crates/connectors-host/src/local/runtime.rs:595-612`); connection, evidence, approval and invocation all assume a secret. Admitting a credential-less profile changes that rule, so it is decided in `architecture-decision-record:credential-less-public-reads` (proposed) before any implementation. This story is implemented only after that record is accepted.

## Acceptance (once the record is accepted)

- The adapter specification kind and the shared ESS model declare a credential-less profile and a per-operation `public` mark; a profile with no credential is refused at load unless every operation it serves is `effect: read` and marked public. Spec first.
- The six operations above run with no connection secret; a test proves an unmarked GitHub operation (`notifications.list`) and any write are refused on that profile with the host's existing credential refusal.
- Conditional requests: the provider stores the `ETag` of each answer it serves on this profile, sends `If-None-Match` on the next identical request, and on a 304 returns the stored answer byte-identical and marks it `not_modified`; a test counts that a 304 run consumes no rate-limit budget (`x-ratelimit-remaining` unchanged in the fixture).
- A 403 or 429 with `x-ratelimit-remaining: 0` is `rate_limited` with the reset time from `x-ratelimit-reset`; never retried in a loop.
- `docs/catalog-github.md` documents the anonymous profile, the limit, the ETag store's location and bound, and the config block and commands for a local install, with the release link.
