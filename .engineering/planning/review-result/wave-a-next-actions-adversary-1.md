---
format: aep.planning-md/3
id: review-result:wave-a-next-actions-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on connection-next-actions-no-cycle (wave A, issue 181)
relations:
- reviews: story:connection-next-actions-no-cycle
revision: 1
---
```
unit: story:connection-next-actions-no-cycle (impl/connection-next-actions-no-cycle a008ef18d, plus uncommitted adversary cases in the worktree)
verdict: red
cases: executed 3→6, red 3
origin: introduced 0, pre-existing 0, undecided 3
wrote-outside-worktree: <scratch>/units/connection-next-actions-no-cycle/scratch/{adv_cases.rs,build.log,adv-revalidate.log,adv-reconnect.log,adv-connect.log,adv-suite-before.log,adv-suite-after.log}
needs-coordinator: yes
```

1. `git diff --stat`: `apps/connectors/tests/connection_next_actions.rs | 67 +++++-` (one test file; no non-test path). The `walk` helper now calls a new `walk_from(start, limit)`. Its body is unchanged except the start action, so the existing cases still walk from `revalidate_connection`. Three new `#[ignore]` custody cases, all red:
   - `adversary_the_walk_from_revalidate_satisfies_the_story` (red): `revalidate` answers `create_connection`, and that `connect` is refused with `lifecycle_conflict` / `revoke_connection`. The walk is revalidate, connect, revoke, connect: 4 commands.
   - `adversary_the_walk_from_reconnect_satisfies_the_story` (red): reconnect (`lifecycle_conflict`, `revalidate_connection`), revalidate (`not_granted`, `create_connection`), connect (`lifecycle_conflict`, `revoke_connection`), revoke, connect: 5 commands.
   - `adversary_the_walk_from_connect_satisfies_the_story` (red): connect (`lifecycle_conflict`, `revalidate_connection`), then the same 4 steps: 5 commands.
   - Single-case red line, revalidate: `panicked at connection_next_actions.rs:501:9: from revalidate, step 1 (connect) ran the command step 0 named and was refused by the state it led to` and `test result: FAILED. 0 passed; 1 failed`. Same panic for reconnect (step 2) and connect (step 2).
2. Suite (`cargo test --locked -p connectors --test connection_next_actions -- --include-ignored`):
   - Before, with `--skip adversary_`: `ok. 3 passed; 0 failed`, exit 0.
   - After: `FAILED. 3 passed; 3 failed`, exit 101.
3. Findings:
   - **Acceptance fails** (`crates/connectors-host/src/local/owner/supervisor.rs:2106`). A refused upgrade's revalidation sets `reconnect`, so it answers `create_connection`. The instance still holds the connection, so that connect is refused naming a revoke.
     - "Four, not three" is not forced by the one-connection rule. Answering `revoke_connection` there gives revalidate, revoke, connect: 3 commands.
     - The story's Outcome says every next action must be one "the state it leads to accepts", and this one is not, on every path the issue shows.
     - The unit's test allows 4 commands and checks only the revalidate start.
     - What reaches it: the documented `connections revalidate` after any configuration change whose new provider refuses the credential, which is the issue's own scenario.
     - Origin `undecided`: the base was not run. The diff does not touch this line.
     - Fix: set `error.revoke` instead of `error.reconnect` when the instance still holds the connection.
   - **"Refusal naming the probe" is unchecked** (`apps/connectors/tests/connection_next_actions.rs:450`). The final refusal carries only `code=not_granted`, `stage=admission`, `next_action=retry_explicitly` and an acquisition id. Nothing in it names the probe, and the test accepts it anyway. `retry_explicitly` re-runs the same command with the same answer; the walk stops there only because it breaks on that action. Judgement, `undecided`.
   - **CI never runs the cases** (`.github/workflows/rust-gate.yml:65`). The two new cases need the CUSTODY prerequisite, and CI runs only the `--only-prerequisite provider` lane. This is the same policy as `failed_connect`; note, `undecided`.
   - **Coordinator:** my three new `#[ignore]` cases are not classified in `crates/connectors-build/src/ignored.rs`. `connectors-build ignored` will count them as unknown until they are classified there or dropped.
4. Could not break:
   - `pending_reason` covers every way `observe`/`readiness` produces `pending`, and is `None` otherwise (Stale, Revoked and Ready all go through the `state == Pending` guard).
   - The `cli.yaml`, `semantics.md`, `cli.md`, `binding.json` and model pages agree. The contract already said connect/reconnect refusals keep `retry_explicitly` (semantics.md:1825).
   - `held`/`reconnect` never come with `NotGranted`, so the new mapping's flag exclusions cannot mis-fire.
5. Paths outside the worktree: the ones in the header, all in the assigned scratch directory. Lease `adv-cz-u181` released.

```findings
[
  {"file": "crates/connectors-host/src/local/owner/supervisor.rs", "line": 2106, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "undecided", "message": "a revalidation refused under an upgrade answers create_connection while the instance still holds the connection, so the connect it names is refused with lifecycle_conflict/revoke_connection and the walk takes 4 commands from revalidate and 5 from reconnect or connect; answering revoke_connection ends it in the 3 the Acceptance requires"},
  {"file": "apps/connectors/tests/connection_next_actions.rs", "line": 450, "category": "judgement", "severity": "warning", "verdict": "CONFIRMED", "origin": "undecided", "message": "the terminal refusal is accepted on code not_granted plus retry_explicitly alone, and carries nothing naming the failed probe as the Acceptance asks"},
  {"file": ".github/workflows/rust-gate.yml", "line": 65, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "undecided", "message": "both new cases need custody and CI runs only the provider-only ignored lane, so no CI job ever executes the regression test for this story"}
]
```
