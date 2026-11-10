---
format: aep.planning-md/3
id: story:windows-clock-check-bracket-flake
kind: story
status: active
title: Clock-check tests of the local CLI sometimes fail on Windows CI
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: inferred
  path: apps/connectors/tests/local_cli.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:39Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:40Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
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

## Wave status

Merged 01b658fb29 (commit 18f382ab3e), 2026-11-10.

- Cause, reproduced on Linux: the fixtures sign one fixed instant, while the check reports the time of its own reading, so a source's lower bound is the signed instant minus radius plus the time since its reply. A reading more than the radius after a reply (a stalled runner) puts the lower bound above the signed instant; the test asserted it never does. Red: a 1.2 s late source gave a lower bound 182 ms above the instant.
- Run 40355771253 failed differently (source unreachable, below quorum); the fixture warm-up of story:clock-quorum-windows-unreachable-flake addresses it. Run 40955636193's failed-job log is empty because its latest attempt succeeded.
- Fix in the test: the lower bound may exceed the instant by the command's own measured wall time; the upper bound is unchanged. local_cli 34 passed.
- Not met: repeated passes on the Windows job. The pull request CI runs it once; the story stays active until that line is observed.
