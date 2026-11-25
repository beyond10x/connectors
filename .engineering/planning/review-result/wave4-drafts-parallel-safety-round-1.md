---
format: aep.planning-md/3
id: review-result:wave4-drafts-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1, three drafts of wave 4
relations:
- reviews: story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs
- reviews: story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run
- reviews: story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout
revision: 1
---
Verdict: needs-revision for each of the three drafts. Read on the branch of pull request
309 at 7f6d60b4e (the MCP draft) and on the branch of the four stories at 7e0bb9401 (the
other two).

Verdicts: all three `needs-revision`. The three drafts do not collide with each other. No build
or test run by the critic.

## story:two-mcp-tests-wait-out-the-20-s-request-budget-a-listed-tool-needs

1. blocker, story line 9: writes `BUDGET` (`apps/connectors/tests/security_mcp_deadline.rs:730`,
   the on-time test at `:889`) and `REQUEST_MS`
   (`apps/connectors/tests/mcp_tool_call_shapes_adversary.rs:43`, the walk's `delay_ms` at `:423`),
   which story:seven-mcp-tests-use-a-4-s-request-budget-and-shorter-holds claims alone (its lines
   141, 243). No `depends_on`, no scope.
2. blocker, line 63: the release-build test names no home; the lane's only release step is
   `-p connectors-sdk … release_build_` (`.github/workflows/slow.yml:113`, asserted at
   `crates/connectors-build/src/ignored.rs:2822-2827`). A test in `connectors` edits both files.
3. warning, line 48: the override variable is not named; without the prefix
   `CONNECTORS_TEST_OVERRIDE_` the owner spawn in `transport.rs` is edited, which
   story:four-idle-exit-tests-no-longer-span-several-5-s-recovery-sweeps writes.
4. warning (inferred), line 70: a unit test of the comparison at `projection.rs:311` would land in
   `apps/connectors/src/mcp/tests.rs` (line 135 holds `execution_ms":20000`); scope lists neither
   it nor the overlap with the inferred scope `apps/connectors/src` of
   story:mcp-outbound-stdio-runtime (draft; its lines 14, 26, 28 also hold `Cargo.lock` and
   `crates/connectors-build/src`).

## story:the-sixteen-ess-model-tests-read-schemas-generated-once-per-run

5. blocker, lines 73-76: acceptance 4 reads forge timings that include the effect of the MySQL
   fixture story (`ess_model.rs:9-10` includes `mysql/fixture.rs` by `#[path]`); no `depends_on`.
6. warning, lines 49-54: Change 1 names no per-process name for the sibling temporary directory and
   no crate name in the key; `CARGO_TARGET_TMPDIR` is `target/tmp`, shared by every package of the
   tree (`connectors-build` tests use `target/tmp/adversary-docs-audit/*` and `mcp-pin-pass2-*`;
   grafana, loki and prometheus each carry an `ess_model.rs`).

Read: within one run 16 nextest processes can all find the directory missing and all generate;
the final path is written by one `rename`. No `target-dir` is set, so the directory is per tree.

## story:a-provider-budget-that-ends-in-an-unanswered-tls-handshake-is-marked-as-the-providers-timeout

7. blocker, lines 9, 82-83: writes `crates/connectors-host/src/http.rs` and
   `crates/connectors-host/tests/provider_timeout_adversary.rs`, both written by the five-tests
   story (its lines 181, 183, 190); acceptance 3 reads tests it owns; only `informed_by`; `waves`
   places the draft in wave 1. Needs `depends_on`.

## Pairs and surfaces

- Draft 3 × draft 2: `ess_model.rs:13` imports `connectors_host`; an `http.rs` change rebuilds the
  `ess_model` binary (rebuild coupling, no collision).
- Clear: `Cargo.lock` (no new dependency in the drafts as read), `.config/nextest.toml`, the lane's
  inventory (`slow_lane_` by name, `ignored.rs:580-585`), generated files.
- Not established: `test-binary-per-package-rest` (draft, no scope) would move every test file
  under `tests/main.rs` and change binary ids such as `--test ess_model`; unassessed.
