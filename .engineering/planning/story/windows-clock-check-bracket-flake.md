---
format: aep.planning-md/3
id: story:windows-clock-check-bracket-flake
kind: story
status: draft
title: Clock-check tests of the local CLI sometimes fail on Windows CI
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Defect

On `windows-latest`, the clock-check tests of the local CLI (`apps/connectors/tests/local_cli.rs`)
sometimes fail with an observation that does not bracket the fixture's time:

- On 2026-10-29, run 40955636193 ("host and CLI on windows-latest", main at the v0.59.2 merge
  db6050273) failed `clock_check_reports_each_source`, with "assertion failed:
  observation["lower_unix_ms"].as_i64().unwrap() <= truth" (local_cli.rs:189). The same tree
  passed that job on PR 219.
- Run 40355771253 failed `clock_check_uses_current_configured_key_without_metadata_or_service_start`.

Neither run touched the clock code or the tests.

## Outcome

The cause is named from a reproduction or the job's logs: the clock bracket, the fixture's
timing on Windows, or the assertion. Either the clock code or the test is fixed.

## Acceptance

- A cause, with the evidence for it.
- The Windows job passes the clock-check tests repeatedly, with no `#[ignore]` and no retry.
