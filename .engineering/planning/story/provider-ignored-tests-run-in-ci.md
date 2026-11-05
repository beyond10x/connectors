---
format: aep.planning-md/3
id: story:provider-ignored-tests-run-in-ci
kind: story
status: implemented
title: Tests that need a built catalog provider run on every pull request
relations:
- informed_by: story:connectors-do-short-invocation
- serves: vision:independent-contract-adapters
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T05:34:02Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-04T08:46:47Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-11-05T01:33:53Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
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

## Delivered (wave 2026-11-04, v0.66.0)

Unit commit ae459799e and adversary cases ec568274b, merged in 5adf08852 on `wave/2026-11-04-connectors`;
local staged gate green at ca1ef2f4d.

- `crates/connectors-build/src/ignored.rs`: `--only-prerequisite <set>` (`provider`, `cli`) keeps only
  the ignored tests whose every prerequisite is in the set; with `--required` an empty selection refuses.
- `.github/workflows/rust-gate.yml`: a step in the `gate` job after the repository gate runs
  `connectors-build ignored --required --only-prerequisite provider`.
- `docs/development.md` § Ignored tests describes the option and the CI step.

Shown: the four provider tests run by name with `CONNECTORS_TEST_CATALOG_PROVIDER` set, and with the
describe short-circuit in `apps/connectors/src/local/invoke.rs` reverted,
`describe_of_a_definition_needs_no_connection` fails. Not shown locally: the full CI step, which compiles
every workspace test; its first run is the pull request's.
