---
format: aep.planning-md/3
id: review-result:connectors-wave-2026-11-04-u4-adversary
kind: review-result
status: active
title: Adversary pass on the provider-only ignored selection (wave 2026-11-04)
relations:
- reviews: story:provider-ignored-tests-run-in-ci
revision: 1
---
## Adversary pass: unit u4-provider-ignored-ci (provider-ignored-tests-run-in-ci)

Verdict: holds. Added 6 cases in the `mod adversary` block of `crates/connectors-build/src/ignored.rs`,
all passing on ae459799e; committed as ec568274b. Pre-existing findings: none.

Cases:

- The `rust-gate.yml` ignored step, parsed with the real command line, selects exactly the four tests
  `docs/development.md` names and runs each by name; the guide shows the same command line the workflow
  runs.
- An unreviewed ignored test beside the four refuses the CI step before any execution.
- A provider the build did not supply refuses the step as a missing prerequisite instead of skipping.
- A disposable test with no prerequisite qualifies, as the guide says; narrowing never admits a live or
  timing test.
- `cli` maps to `CONNECTORS_TEST_CLI` and `provider` does not admit it.

Not shown locally: the full `connectors-build ignored --required --only-prerequisite provider` run, which
compiles every workspace test; its first execution is the pull request's CI run.
