---
format: aep.planning-md/3
id: story:local-cli-capacity-probe-flake
kind: story
status: draft
title: The build probe against a full owner reports not_running instead of capacity in 2 of 5 runs
relations:
- serves: vision:independent-contract-adapters
revision: 2
---
## Defect

`adversary2_the_build_probe_against_an_owner_at_capacity_reports_capacity`
(`apps/connectors/tests/local_cli.rs`, assertion at :1409) answers `ok:not_running` where
`capacity` is expected, about 1 to 2 s after the owner was filled with 32 clients, well before the
owner's 10-second greeting deadline that the test's comment names as the only way a slot frees.

Observed on 2026-10-29 on Linux, on `main` `f4df1c4f8` (ESS 0.75.0), without load from other
suites: 2 of 5 runs of `cargo test --locked --offline -p connectors --test local_cli -- --exact
adversary2_the_build_probe_against_an_owner_at_capacity_reports_capacity` failed (elapsed 1.06 s in
the failing full-gate run). The same tree with only the ESS pin moved failed 3 of 5 runs, and the
full repository gate failed on it once.

## Outcome

The test passes only when the CLI reports `capacity` against a full same-build owner, and its
verdict does not depend on scheduling. If the CLI or the owner, not the test, frees a slot early
(a held client closed, a probe connection counted differently), the code is fixed and the cause is
named.

## Acceptance

- The cause of `ok:not_running` within 2 s of filling the owner is named with a reproduction.
- The test passes 200 of 200 runs on Linux under load (`nice -n 19`, at most 4 workers).
- No `#[ignore]`, retry or longer deadline without the named cause.
