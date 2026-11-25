---
format: aep.planning-md/3
id: review-result:test-step-stories-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2, the test-step story set
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: needs-revision.

Round 2 of the parallel-safety critic on the 18 drafts of
`story:the-gates-test-step-ends-within-120-s-on-the-forge`, read on 2026-11-25 on the branch
`plan/test-step-stories`. Read-only, nothing was run. Five findings, all warnings. Every collision
inside the set is ordered by an edge or stated as disjoint regions, and the regions were checked
against the tree.

## Findings

1. `story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition`: its
   digest covers every `.rs` file under `crates/connectors-catalog/src/` and everything under
   `definitions-0.5.2/`, and the body states no rule for who regenerates when an input changes.
   Eight drafts outside the set write those inputs by recorded scope: catalog-zendesk-reads,
   definition-amendments-pinned-by-digest, definition-files-in-user-config,
   definitions-pin-covers-catalog, slack-sdk-upstream-import, definitions-default-write-guard,
   agent-tool-projection, slack-four-profile-definition-generated. Ten more name a file in that
   tree as their defect site (inferred). Change: the body says "who changes an input regenerates in
   the same change", and the fast check's failure prints the regeneration command.
2. `story:the-lane-slow-runs-the-ignored-timing-family`: it says
   `story:test-binary-per-package-rest` is harmless because the lane rule reads function names. The
   commands of six stories of the set name `--test <binary>` and break when that story merges:
   `-p connectors --test connection_lease_adversary` and `--test connection_lease` (eleven),
   `--test owner_slow_start` (slow-owner), `--test local_cli` (100-run),
   `--test security_mcp_deadline` (seven-mcp), `-p connectors-host --test local_foundation`
   (four-host), `--test provider_timeout_adversary` and `-p connectors-sql --test protocol`
   (http and sql). The nextest ids of the same stories go stale too. Change: say who rewrites them:
   the later of the two stories, or an edge.
3. `story:where-do-the-10-s-of-the-identification-retry-loop-test-go`: question 2 reads the override
   of the connect wait that
   `story:seven-cli-tests-of-a-slow-owner-start-no-longer-wait-out-the-10-s-connect-wait` creates,
   with no edge. Change: add the edge, or say question 2 is answered with a throwaway probe.
4. `story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps`: the scope names only the
   test at `local_cli.rs:1959`. The new sweep variable must reach an owner that the shared helper
   `start_owner_holding_startup` starts (`local_cli.rs:972-1010`); that helper has 17 callers, among
   them `start_owner`, which the 100-run test uses. Change: the scope names the helper region and
   states that no signature and no caller changes.
5. `story:seven-cli-tests-shorten-their-waits-or-a-production-wait`: the same shape.
   `clock_check_below_quorum_refusal` runs the CLI through `clock_unavailable` and `command_from`,
   shared with other clock tests and every CLI test of the file. Change: the scope names the region
   it edits.

## Round-1 findings

Resolved: `slow.yml` (only the lane story writes it); the eleven-tests story's missing files; the
spawn's environment list (the helper story owns it by prefix rule); the catalog reader beside its
producer; `nextest.toml` (the eighteen-tests story alone); `refresh/tests.rs` named with its region;
the `#[ignore]` mechanism; the release-build job; the new files' paths; one writer each for
`owner_slow_start.rs` and `security_mcp_deadline.rs`; `clock.rs` (edge). Partly: the clock-offset
seam; the eleven-tests story's scope is provisional until the lease question is answered. Not
resolved: none.

## Files written by two or more stories

- `crates/connectors-host/src/local/owner/transport.rs`: helper, idle-exit, slow-owner, seven-cli,
  eleven; a total order by edges; disjoint regions.
- `apps/connectors/tests/local_cli.rs`: 100-run against idle-exit and against seven-cli: no edge,
  stated disjoint, verified. Idle-exit against seven-cli: ordered through slow-owner. The shared
  helper regions are findings 4 and 5.
- `apps/connectors/tests/support/mounted.rs`: six-cli against eleven, no edge, regions stated and
  verified.
- `refresh/tests.rs`: five-host against eighteen, no edge, regions stated and verified.
- `test_fixtures.rs` (new): eighteen creates it, six-cli edits it, ordered by an edge.
- `crates/connectors-sdk/src/lib.rs`: helper against eleven, ordered by an edge.

Named in a Change section and missing from the scope: `adapters/catalog/tests/local_runtime/cli_journey.rs`
calls `finish_filtered_owner`, which the eight-tests story changes; `Cargo.lock` changes with the
eighteen-tests story's new dependency.

## Waves

No two stories of the set in one wave collide. Wave 1: five-host and the lane story. Wave 2:
four-host and the helper story. Wave 3: eight, eighteen, http and sql, seven-mcp, 100-run,
catalog-provider. Wave 4: idle-exit and catalog-crate. Wave 6: slow-owner and six-cli. Wave 7:
seven-cli. Wave 8: eleven. The two question stories have no scope and sit in no wave.

Collisions with stories outside the set, which the tool orders: the lane story against
claude-getting-started-tutorial and mounted-custody-kind-secret-volume (`ignored.rs`,
`docs/development.md`); the eighteen-tests story against catalog-zendesk-reads
(`connectors-build/src/main.rs`) and definitions-pin-covers-catalog (`connectors-build/Cargo.toml`).

## A first wave

- `story:five-host-unit-tests-shorten-the-waits-they-chose` has no edge and can start now.
- The two question stories have no edge and write no file of the repository.
- The four-host and 100-run stories depend on the lane story, as do the seven-mcp, eight-tests,
  catalog-provider and helper stories.
- Without edges and pairwise safe: the lane story, five-host, eighteen, the two question stories.

## Not established

- The paths of the files the stories will create.
- The eleven-tests story's scope after the lease question is answered.
- How `story:test-binary-per-package-rest` is cut.

## Outside this critic's role

- The lane's acceptance 3 lists `slow_lane_` tests that only its dependents create, so it is empty
  at its own merge.
- The eighteen-tests story gives `connectors-build` a dependency on `connectors-host`; whether that
  feature graph holds was not checked.
