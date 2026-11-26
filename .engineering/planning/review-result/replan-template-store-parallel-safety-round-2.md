---
format: aep.planning-md/3
id: review-result:replan-template-store-parallel-safety-round-2
kind: review-result
status: active
title: Parallel-safety critic, round 2, the re-plan of the template-store story
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: **needs-revision**

Everything below was read at commit 42f599466. The working tree matches it: `git status` is empty and `aep plan artifact waves --kind story --status draft` agrees with the parent's wave statements (S9e in wave 1, N7 in wave 4, S9d in wave 6).

## Round-1 findings

1. **Resolved.** N7 exists, S9d has a `depends_on` edge to it, and S9d's scope names the reason ("Without that story this pull request is red in the gate"). At 42f599466, `er.rs:5095-5097` is still the only cited line in `cli.yaml`, and the test's regex `er\.rs:(\d+)-(\d+)` is still the one that goes red.
2. **Resolved.** S9d's scope section gives the order with the three drafts as prose, with its reason. It also says a regenerated bundle changes the cost of one build, not the counts or the formula.
3. **Resolved.** S9e names ten binaries and 84 test functions. Counted: 83 test functions in the nine `adapters/sql/tests` files, plus 1 in `crates/connectors-conformance/tests/incident_call_join.rs`.
4. **Resolved.** S9e's scope states that `five-tests…` writes `mysql_protocol.rs`, which includes the fixture, so the two stories write different files and share one binary. Nothing else non-implemented names `adapters/sql/tests` or `mysql/fixture.rs`.
5. **Resolved.** N6 has `depends_on` S9d.
6. **Resolved.** N6's scope puts timers only in a worktree made for the measurement, "never in the tree of a story that writes `er.rs`, `metadata.rs` or a test file". It also re-reads the line citations on the head that holds S9d. N5 has `depends_on` S9d, and its question 5 says it is the one question that reads a run made after a story landed.
7. **Resolved in part.** "Base" is now one formula, re-read after a merge of main, and a job over the bound is not re-run. "The pull request's base commit" is still not tied to what the job checked out (finding 3).
8. **Resolved.** The parent states that its acceptance has no edge to S9d or S9e, with the reason.

## New findings

1. **Blocking. N7 tells the implementor to regenerate `inputs.json` as one pool job.** N7 point 4 says to regenerate with `connectors-build metadata-conformance emissions`, and its "Not in this story" says this "runs on the build host as a pool job that names its time (30 minutes at most)". `docs/development.md` lines 92-97 say the opposite. A part takes 8 to 27 minutes, "the whole run does not fit one job", and the run is three jobs (`--part component`, `--part connectors.oauth2`, `--part connectors.approval_issuers`), each of which also rewrites `inputs.json`. A single 30-minute job would be killed with exit 124, and the folder rule says not to start it again unchanged. This also sets the price of "whoever merges second regenerates": up to three jobs of that length per merge. N7 should name the three `--part` runs and say each shared-record merge pays them. The same command in `docs/development.md` also runs on "this workstation", so N7 should say the jobs run on the build host.

2. **Note. N7's list of who shares `inputs.json` is incomplete.** It states its method ("from the `scope` of each story's frontmatter"), so it is incomplete rather than wrong. Checked against the store.
   - **Count.** It says "26 of them under `ess/domains/`". The file has 27 (`grep -c`, and 27 files in `ess/domains/`). Lines 52 and 56 are right.
   - **Text-only sharers.** Nine drafts record no scope and say in their text that they edit `ess/domains/cli.yaml`:
     - `a-clock-refusal-of-issue-and-invoke-names-its-sources`
     - `custody-credential-file-delivery`
     - `operations-list-text-match`
     - `operations-usage-report`
     - `refusals-name-the-field`
     - `setup-check-reports-weakened-guarantees`
     - `tool-contract-serving`
     - `upgrade-moves-catalog-adapters`
     - `ess-format-paging-views`

     They are unassessed rather than safe. S9d gives two lists (recorded scope, text); N7 gives one.
   - **Other inputs.** `inputs.json` also hashes `crates/connectors-build/src/metadata_emissions.rs`, `metadata_entities.rs`, `ess/components.yaml`, `ess/system.yaml`, `crates/connectors-spec/toolchain.json` and the ESS crates of `Cargo.lock`. Four drafts record `crates/connectors-build` or `crates/connectors-build/src` as a directory: `mcp-outbound-stdio-runtime`, `slack-four-profile-definition-generated`, `slack-method-token-scope-table` and `slack-sdk-upstream-import`. Whether they touch those two sources is unassessed.
   - **History.** "Three commits moved the numbers" is short. `c97cb5660` and `df1f2c5b1` did the same on 2026-11-23 and 11-24, and each also changed 4 lines of `inputs.json`. That supports N7's claim about the record, so it could be cited.

   What settles it: add the text list and the build-source line to N7's scope section, and correct 26 to 27.

3. **Note. S9d acceptance 4 does not say what "base commit" means for a pull-request gate job.** `rust-gate.yml` runs on `pull_request`, so the job checks out the merge of the branch with the main tip at job start. If main moved without a push, "base" is read at one commit and the job ran on another. The ruleset's "up to date" rule probably makes them equal at merge time. One sentence would settle it: base is the first parent of the merge commit the job tested, and the pull request records it.

4. **Note. Timing runs on the one build host can overlap, and no story says they take turns.** S9e acceptance 1 is three runs against three runs, "in turns", with a 3.0 s rule and "refuted twice: nothing is merged". N6 items 9 and 11 "wait for nothing" and also time on the build host. S9e does not record host load beside each run; N6 item 11 does, for its own runs. Load makes the base runs longer (the 7.0 s floor guards that direction) and also makes the "with point 1" runs longer, so a loaded host can falsely refute the cause. Adding one sentence to S9e would settle it: no other timing run of the set is running, and the load is recorded per run.

5. **Note, S9d scope. "Stories of the set that name the file: only the archived `eighteen-tests…`" is not true as written.** `store-tests-end-within-60s-under-load` (line 105, "the unchanged `er.rs`"), the parent, N6 and N7 all name `er.rs`. None writes it. The sentence should say "write".

For another critic (acceptance or design): S9d point 5 says `definition_bundle()` has "four in the test module (lines 6119, 6460, 6539 and 6784)". There are nine (also 6877, 7078, 7534, 7591 and 8288). The conclusion "the build is its only caller outside the test module" still holds, because the other call sites outside the test module are 1170 and 4328.

## N7 and S9d in sequence

- They share only the edge. `fn update_command(` is at line 4910 and `fn removal_command(` at line 5093, each at column 0 and each the only match in the file. Neither body calls `registry()` or `definition_bundle()`; the only `registry()` and `definition_bundle()` calls between lines 4900 and 5200 are none.
- S9d's points are at lines 901, 992, 1170, 1176, 2377, 2484, 4328 and 4334, all outside those two bodies. The new test goes in `mod tests`, indented, so the body finder's "first-column `}`" rule cannot catch it.
- The bundle `entity-runtime-definitions.json` carries `source_digest` and `synthesis_digest`, so it was checked whether a comment-only edit of `cli.yaml` would force a regeneration of the bundle. In `df1f2c5b1` and `c97cb5660`, comment-only edits of `cli.yaml` changed `inputs.json` and not the bundle. N7's claim holds.

## Active-story and branch checks

- **No active story has a recorded path under `ess/`.** The `scope` of all 214 draft and active stories was read: 296 path lines, and the only `ess` entries are the drafts named above.
- **Recorded scope of the shared files** matches N7 and S9d on every point read: three drafts record `cli.yaml`, `admission-checks-token-scopes` records `ess`, and `spec-models-operation-admission` records `er.rs` and the bundle. `mcp-outbound-stdio-runtime` records `crates/connectors-host/src/local`. `connections-list-answers-through-the-owner` (active) records nine paths, none of them `metadata/er.rs`; "owner.rs" contains "er.rs", so a loose grep misleads. Its text says "this story edits no ESS file" (line 196), consistent with N7's claim. `post-commit-metadata-failure-is-outcome-unknown` cites lines 4115-4122, 4137-4144 and 4197-4207 and records no scope.
- **Test-step set and `crates/connectors-build/tests/`.** Only `story/debug-override` writes there, adding `debug_override_reads.rs` and one `mod` line to `main.rs`. N7 edits an existing module and does not touch `main.rs`, so there is no textual clash. `definition-tests-read-pinned-release` (draft) writes `catalog_binary_model.rs` and `catalog_identity_model.rs`, not N7's file.
- **In-flight branches.** `git diff --name-only f9f263a2...<branch>` for `debug-override`, `mcp-budget`, `released-lowering`, `four-host-lock`, `six-cli-link`, `five-host-waits`, `lane-slow` and `eight-tests-drop-waits`: none writes `er.rs`, `cli.yaml`, `auth_bindings.yaml`, `cli_spec_mapping_adversary.rs`, `inputs.json`, `fixture.rs`, `mysql_protocol.rs` or `incident_call_join.rs`.
- **Content coupling, harmless.** `debug_override_reads.rs` scans every `src/` `.rs` file for reads of override variables, `er.rs` included. S9d adds no environment read.
- **Older branches** (`story/read-uses-in-memory`, `story/host-tests-own-process`, `story/mysql-loki-reads` and others) do write `er.rs` or `cli.yaml`, but their stories are implemented and squash-merged. For `host-tests-own-process`, `git diff` of `er.rs` against main is empty.

## Registry draft's scope section

- **Lists.** Both lists of who names `er.rs` match the store. The one gap is finding 5.
- **No digest of `er.rs`.** True. `inputs.json` has five non-ess paths and `er.rs` is not one; `metadata_conformance.rs` mentions `er.rs` only in comments.
- **Bundle.** `metadata_conformance.rs` line 40 `include_bytes!` the bundle, and `metadata_entities.rs` lines 183-198 write and check it. That is a compile-time dependency and a regeneration by whoever changes the definitions (`owner-observe-request-is-modelled-in-ess`, `spec-models-operation-admission`). S9d writes neither, so it creates no shared record for S9d.

## S9e and the time statements

- **Fixture-sharing binaries.** The 9 plus 1 are right, and `five-tests…` adds no write to the fixture. Its `mysql.rs` and `lib.rs` changes touch the query timeout, not what `ess_model` waits on.
- **Fixed 2.0 s bound.** It is independent of S9d, because the eleven tests are not in `ess_model`.
- **Registry acceptance 4.** The re-read after a merge of main works, apart from finding 3.

## N5 and N6

- **N5's new edge** forms no cycle: parent, N5, S9d, N7. The waves print without error.
- **N6 items 12 and 13, the acceptance, and the scope's timers rule** raise nothing for this lens. One more line would help: a story drafted from items 1 to 8 or 13 may write `issue.rs` or `registry/tests.rs`, which `store-tests-end-within-60s-under-load` records. The tool will show that when the draft records its scope.

## Not checked

- Whether the forge's merge-ref checkout is what the job used for any past job: only the workflow file was read, no forge run.
- Whether any text-only draft will really edit `cli.yaml`: one to six lines of text were read for each, not the plans.
- The 8 to 27 minute regeneration time: read from `docs/development.md` and not run, under the rules of the brief.
