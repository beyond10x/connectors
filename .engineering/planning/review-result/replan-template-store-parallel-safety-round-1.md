---
format: aep.planning-md/3
id: review-result:replan-template-store-parallel-safety-round-1
kind: review-result
status: active
title: Parallel-safety critic, round 1, the re-plan of the template-store story
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: needs-revision. Read at commit 198fc306b. S9d is the registry draft, S9e the MySQL
fixture draft, N6 the question story of the re-plan, N5 the question story of the tests under
5 s. Saved from the critic's report; wording shortened, findings complete.

## Findings

1. Blocking. S9d says "No other file is written". `ess/domains/cli.yaml` line 2218 cites
   `er.rs:5095-5097`; the test `the_er_rs_citation_lands_on_the_cursor_expiry_path`
   (`crates/connectors-build/tests/cli_spec_mapping_adversary.rs`) holds those lines. Every
   point of S9d is above line 5095. A re-point edits `cli.yaml`, whose sha256 is in
   `crates/connectors-build/generated/metadata-emissions/inputs.json`. `cli.yaml` is in the
   scope of the open drafts `owner-observe-request-is-modelled-in-ess`,
   `definition-files-in-user-config`, `timeout-refusal-says-how-long-it-waited` and
   `admission-checks-token-scopes`.
2. Note. The order of S9d with `spec-models-operation-admission`,
   `mcp-outbound-stdio-runtime` and `post-commit-metadata-failure-is-outcome-unknown` (no
   scope; cites the region of `persist_batch`) is prose only.
   `connections-list-answers-through-the-owner` (active) reads `er.rs` and does not write it.
   A regenerated bundle changes S9d's figures.
3. Blocking. S9e: a tenth binary includes the fixture
   (`crates/connectors-conformance/tests/incident_call_join.rs`).
4. Note. `five-tests-no-longer-wait-out-the-http-and-sql-client-timeouts` writes
   `adapters/sql/tests/mysql_protocol.rs`, which includes the fixture: same binary, no file
   clash.
5. Blocking. N6 needs a `depends_on` edge to S9d.
6. Note. N6's timers belong in a scratch worktree of their own; its line citations are read
   again on the head that holds S9d. N5 reads S9d's gate job and has no edge to it.
7. Blocking. "The newest gate job of its base commit" is ambiguous when S9d and S9e are open
   together. Each pull request records the job and commit of its "before" on the base of its
   last push and reads it again after a rebase; S9e states that its bound does not depend on
   S9d.
8. Note. The parent's acceptance reads the forge step that S9d and S9e change, and has no edge
   to them.

## Checked and found correct

- S9d and S9e share no file, no lockfile and no test binary.
- No committed digest covers `er.rs` or `fixture.rs`.
- None of the four branches in flight writes `er.rs` or `fixture.rs`.
