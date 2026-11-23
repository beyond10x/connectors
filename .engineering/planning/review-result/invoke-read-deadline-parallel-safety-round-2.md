---
format: aep.planning-md/3
id: review-result:invoke-read-deadline-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2, story:invoke-admission-queues-under-load
relations:
- reviews: story:invoke-admission-queues-under-load
revision: 1
---
Both are in the shared helper, so acceptance 1 to 5 can run in `owner_slow_start.rs` without changing the helper.

verdict: approve

No finding blocks implementing this story alone. Checks against revision 27 (`8660e7c27fd`):

- **Scope:** the scope now holds what the change touches. That covers `invoke_short.rs`, `invoke_walks.rs`, `owner_slow_start.rs`, `owner.rs` (the `start_for_read` and lease code at 654 and 1135) and `supervisor.rs`. The generated outputs and `inputs.json` are cited. The Design renumbers the `cli.yaml` citations at :1946 and :2106.
- **Test hooks:** the knob `invoke_ms`, the `/work/invocations` log and `set_knob` already exist in `apps/connectors/tests/support/mounted.rs`. `CONNECTORS_TEST_OWNER_START_DELAY_MS` exists in `transport.rs:1591`. Acceptance 1 to 5 therefore need no new helper.
- **Acceptance 9:** `revalidate_with_retries` is at `supervisor.rs:168`. The `connectors-host` lib tests are inline modules in the same file, so that test needs no scope addition.
- **Overlap:** the "Shares files with" section names every sibling I listed, and `timeout-refusal-says-how-long-it-waited` carries a `depends_on` edge to this story. The Entity Runtime pin is covered by the `inputs.json` regeneration rule. The story is committed, so other worktrees will see its scope once it is on main.
- **Out of scope:** the hand-written flag lists are named as story:mcp-cli-journey-discovery-contract's.

Notes:
1. Acceptance 5 (an owner start delayed 20 s, exit before it greets) depends on `start_for_read` in `crates/connectors-host/src/local/owner.rs`, which is in scope. The 30 s and 40 s waits in `transport/` stay, as the story says. If the implementer finds the start wait lives in `transport.rs`, which is not in scope, the scope needs that path added.
2. `apps/connectors/src/walk.rs` is not in scope, and I did not find the per-page path in it. The page loop appears to be in `invoke.rs` (`Prepared::Walk`, line 638). Acceptance 8 should show whether the flag reaches every page. If it does not, add `walk.rs`.
3. The unscoped and too-wide siblings (story:invoke-exits-promptly-on-sigterm, story:mcp-outbound-stdio-runtime, story:admission-checks-token-scopes and the others) stay unassessed. Nothing else of this repository is in implementation tonight, so this is not a blocker. They need scopes before any later set runs beside this story.
