---
format: aep.planning-md/3
id: review-result:test-step-stories-parallel-safety-round-3
kind: review-result
status: active
title: Parallel-safety critic, round 3, the test-step story set
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Third read, parallel-safety critic, 2026-11-25. Read-only; nothing was run.

## Verdict

needs-revision for the set. One story fails:
story:eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping. The other 18 stories read
are safe to run in parallel. `five-host-unit-tests-shorten-the-waits-they-chose` was not read
again.

## Verdict per story

- `the-lane-slow-runs-the-ignored-timing-family`: approve
- `one-debug-only-override-helper-is-forwarded-to-the-owner-and-off-in-a-release-build`: approve
- `eight-adapter-and-catalog-tests-drop-waits-that-prove-nothing`: approve
- `eighteen-tests-take-a-template-store-or-jail-built-once-per-run`: approve (notes 1 and 3)
- `six-cli-tests-take-jail-binaries-built-once-per-run`: approve
- `seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds`: approve
- `the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition`: approve
- `the-catalog-crate-tests-ingest-the-released-definition-once-per-process`: approve
- `four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps`: approve
- `seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait`: approve
- `five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts`: approve
- `seven-cli-tests-shorten-their-waits-or-a-production-wait`: approve
- `eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping`: needs-revision (finding 1)
- `where-do-the-connection-lease-tests-clock-and-the-30-s-admission-bound-live`: approve (the
  eleven-tests story reads its answer)
- `where-do-the-10-s-of-the-identification-retry-loop-test-go`: approve (the planner reads its
  answer)
- `where-do-the-1298-s-of-the-tests-under-5-s-go`: approve (the operator reads its answer; note 5)
- `store-tests-end-within-60s-under-load`: approve (note 4)
- `the-gates-test-step-ends-within-120-s-on-the-forge` (parent): approve (note 2)
- `four-host-tests-no-longer-wait-out-the-30-s-lock-wait`: approve
- `the-100-run-capacity-stress-keeps-10-runs-in-the-gate`: approve

## Round-2 findings

1. Resolved. The provider story states the rule at its line 116 (whoever changes an input
   regenerates in the same change). Acceptance 5 (line 161) makes the failure print the
   regeneration command.
2. Resolved. Lane story line 189 says the later of the two stories rewrites the eight commands and
   the nextest ids. Note 6 is what is left of it.
3. Resolved. The identification question, line 48, answers question 2 with a throwaway probe and
   says it needs no override and no other story.
4. Resolved. The idle-exit story's scope (line 157) names the helper region (972 to 1024), one
   statement, no signature and no caller change. In the tree, lines 1002 to 1004 are the
   `if let Some(idle_ms)` branch, and `start_owner` passes `None`
   (`apps/connectors/tests/local_cli.rs` lines 935 to 940).
5. Resolved. The seven-cli-shorten story, line 228, names `clock_unavailable` (lines 153 to 171)
   and a new function beside it, and says `command` and `command_from` stay untouched. In the
   tree, `clock_unavailable` is at line 155 and `clock_check_below_quorum_refusal` at line 325.

The round-1 leftovers are closed: `cli_journey.rs` is in the scope of the eight-tests story,
`Cargo.lock` in the scope of the eighteen-tests story, and the six-cli story no longer writes
`support/mounted.rs`.

Placement: 15 stories with cited scope (the eighteen-tests and provider stories also carry
inferred entries for files they create); 0 inferred only; 5 not placed (the 3 question stories,
which write no file by design, `store-tests-end-within-60s-under-load` and the parent).

17 reader-beside-producer pairs were traced, and `depends_on` records all 17:

- lane tests to the lane story (8): four-host, 100-run, seven-mcp, http-and-sql, the provider
  story, the slow-owner story, the eleven-tests story, the helper story;
- override users to the helper (5): idle-exit, slow-owner, seven-cli-shorten, http-and-sql,
  eleven-tests;
- other pairs (4): the catalog crate story to the provider story, the eleven-tests story to its
  question, `store-tests-end-within-60s-under-load` to the eighteen-tests story, seven-cli-shorten
  to five-host.

Files written by two or more stories, each checked in the tree:

- `crates/connectors-host/src/local/owner/transport.rs`: five stories. The edge chain helper,
  idle-exit, slow-owner, seven-cli-shorten, eleven orders them, and the regions are disjoint.
- `apps/connectors/tests/local_cli.rs`: three writers. The 100-run story (1684 to 1727), the
  idle-exit story (972 to 1024 and 1959) and the seven-cli-shorten story (153 to 171 and 325) have
  no edge between them and say their regions are disjoint; confirmed against the tree.
- `crates/connectors-sdk/src/lib.rs`: the lane story, then the helper story, then eleven, ordered
  by edges.
- `crates/connectors-host/src/local/owner/refresh/tests.rs`: the eighteen-tests story edits
  `Connected::build` (314 to 323) and five-host edits `LIFETIME_MS` (995) and one test near line
  1008. No edge, regions disjoint.
- Sole writers: `.config/nextest.toml`, `crates/connectors-build/src/{gate.rs,main.rs}` and
  `Cargo.lock` (eighteen-tests); `slow.yml`, `ignored.rs` and `docs/development.md` (the lane
  story).

## New findings

1. blocker. `eleven-connection-lease-tests-advance-a-clock-instead-of-sleeping`, lines 88 and 180.
   The story says it stays draft because its design is open, and that which of four scope files it
   writes (`transport.rs`, `supervisor.rs`, `connections.rs`, `crates/connectors-sdk/src/lib.rs`)
   "is settled by the answer". The question story has no scope, so `aep plan artifact waves`
   places eleven in wave 8 through the seven-cli-shorten story alone; nothing in the wave order
   holds it for the answer. Approve when the `## Answer` is in the question story and eleven's
   scope is recorded again.
2. note. Parent, line 187: its last change is `test_step` in `gate.rs`, the function the
   eighteen-tests story edits. The order is prose only: no scope, no edge. Record `gate.rs` as
   scope and a `depends_on` to the eighteen-tests story and the question story on the tests under
   5 s.
3. note. The eighteen-tests story against the lane story: the setup script sits in
   `.config/nextest.toml` (line 147) and runs on every nextest run unless it has a `filter`
   (inferred; not run). The lane's fourth step (lane story line 102) is a nextest run of one test
   in `connectors-sdk`, so it would also build the template store and run `ess generate`; likewise
   the helper story's command `cargo nextest run -p connectors-build --test main
   debug_override_reads`. Name a `filter` for the script, or say the lane step pays it.
4. note. `store-tests-end-within-60s-under-load` records no scope, so the waves tool cannot see
   its collisions. Its writes would land on files the eighteen-tests story already edits, and the
   edge covers those. Record the scope.
5. note. The question story on the tests under 5 s, line 59: question 3 computes "from the
   durations of one forge run". If that is a run after the set lands, it reads what all the
   stories produce and has no edge. If it is the arithmetic on the baseline run, say so.
6. note. story:test-binary-per-package-rest (outside the set, no scope) moves the integration
   tests of every package into one binary, which would hit most of the test files this set edits.
   The lane story records only the commands and ids. The rest story's body carries no such
   obligation, and it has no edge to the set.
7. note. Over-wide scope entries: `adapters/catalog/tests/local_runtime.rs` (eight-tests story line
   162) and the two `http_standing_grant*.rs` files in the eighteen-tests story (line 236) say "no
   change is named". Drop them, or the waves tool counts collisions that do not exist.
8. note. The lane's acceptance 2 (line 147) does not say which file its `slow.yml` test lives in.
   If it is in `crates/connectors-build/tests/main.rs`, the helper story's "no other story writes
   it" is wrong. The edge from the lane story to the helper story orders them either way.

## Not checked

- Collisions with stories outside the set.
- `five-host-unit-tests-shorten-the-waits-they-chose` as implemented on its own branch.
- Whether cargo-nextest 0.9.143 runs a setup script and how it scopes one. Whether a hard link
  from `target/` into the jail works on the forge runner.
- Whether `connectors-build` gaining `connectors-host` as a dependency holds in the feature graph.
- Acceptance checkability, the split, and every "after" figure.
