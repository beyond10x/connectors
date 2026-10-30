---
format: aep.planning-md/3
id: story:provider-ignored-tests-run-in-ci
kind: story
status: draft
title: Tests that need a built catalog provider run on every pull request
relations:
- informed_by: story:connectors-do-short-invocation
revision: 2
---
## Problem

Some tests need a built catalog provider. They are `#[ignore]` and run only under
`cargo run -p connectors-build -- ignored`, which supplies `CONNECTORS_TEST_CATALOG_PROVIDER` from
the workspace build (`crates/connectors-build/src/ignored.rs:403`, `:935`). No CI workflow runs
that command; the only job that runs ignored tests is
`.github/workflows/custody-platforms.yml:39`, for `connectors-custody`. So these tests run only
when someone runs them by hand:
- the D6 contract test of story:connectors-do-short-invocation, `describe_of_a_definition_needs_no_connection`
  (`apps/connectors/tests/adversary_d218_u1.rs:687`), which says a `--describe` of a definition
  needs no connection;
- the three tests in `definition_connect_question.rs`.

The D6 test passed by hand on 2026-10-30 against v0.60.0 (1 passed).

## Outcome

Every test the `ignored` command needs only a built provider for runs on each pull request.

## Acceptance

- A CI job builds `connectors-catalog-provider` and runs
  `connectors-build ignored --required` on the tests that need `CONNECTORS_TEST_CATALOG_PROVIDER`.
  Each one is reported as executed, by name.
- Reverting the D6 behaviour, so that a definition's describe asks to connect, fails that job.
