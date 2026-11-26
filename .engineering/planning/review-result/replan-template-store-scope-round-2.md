---
format: aep.planning-md/3
id: review-result:replan-template-store-scope-round-2
kind: review-result
status: active
title: Scope critic, round 2, the re-plan of the template-store story
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: **approve**. An implementor can start N7, S9d and S9e from these files as they are. Five notes remain, none blocking. All figures below were read at commit 42f599466.

## Round-1 findings of the scope role

1. `resolved`. N6 item 12 now owns "the rest". Recomputed from `run5-table.txt`, summing each timer over all sections:
   - `host.feed` 3,467.8 ms, `host.capture_sql` 1,694.2, `m.open_connection` 1,463.9, `m.floor_record` 909.9, and `er.write_checkpoint` plus `er.enable_checkpoints` (470.6 + 227.4 = 698.0). Together 8,233.8 ms, which is the 8.23 s in the item.
   - `er.start` 4,125.2, `er.open.resynchronize` 3,588.9 and `er.complete_snapshot` 4,613.1 sum to 12.327 s. Item 8 holds 8.315 s of that (2,289.3 + 2,547.1 + 3,479.0), which leaves 4.01 s.
   - 27.47 − 5.55 = 21.92, and 27.47 / 117.685 = 23.3 %.
2. Open on purpose (`four-host-tests…` line 118). Not raised again.
3. Open on purpose (the override-helper story, lines 124 to 128). Not raised again.
4. `resolved`. N5 no longer says "The eighteen stories". Its bullet on `Metadata::initialize` now says it was measured.
5. `resolved`. N6 says "Not an item here: the share of REG and PARSE2 in the tests under 5 s", and N5 question 5 takes it. Its figure is the sum of durations of the tests under 5 s in two gate jobs, which is a different quantity (see note 4).
6. `resolved`. The parent's list of the dropped S9a decisions names the setup step: the setting `experimental = ["setup-scripts"]`, about 0.27 s per run, and "No story of the set uses a setup script now". S9d's "Not in this story" also names it.
7. `resolved`. The closing paragraph of `store-tests-end-within-60s-under-load.md` (revision 11) says the registry story does not promise 60 s at load. It says that this story "keeps the change for that test" when a test is still over 60 s, and that the template route is closed (33 to 38 ms per store).
8. `resolved`. The archived story's list for N6 has thirteen entries, and N6's thirteen items match it in the same order. N6's Defect says items 9, 10 and 13 are not about the eleven tests.
9. `resolved`. The parent's "Not claimed by a change" list now holds only the 5 + 6 tests. The 12 tests of S9d and S9e are in "Claimed by a change whose estimate is not confirmed", 11 + 1.

## New findings

1. Note. `the-specification-cites-er-rs-by-function-name-not-by-line.md`, the Defect bullet on `inputs.json`: it says "26 of them under ess/domains/". `inputs.json` holds 27 (lines 40 to 144, step 4, `auth_bindings.yaml` at line 52, `cli.yaml` at line 56). `grep -c '"path": "ess/domains/'` gives 27. Change the figure to 27. Nothing else in N7 depends on it.

2. Note. N7, the Change section. Parent admission: the parent lists N7 under "Without a row, because they cover no test of their own", beside N1 and N2. It says "It claims no test time", and S9d has the edge to N7. That is admitted. Scope: it is one file wider than the smallest that lets S9d change `er.rs` alone.
   - The smallest set is `cli.yaml`, the citation test and `inputs.json`.
   - `auth_bindings.yaml` line 521 is changed only because of the new guard, which fails on any `er.rs:<digits>` under `ess/domains/`. No test holds that citation, and it is outside S9d's need.
   - Say in N7 that the guard is what pulls `auth_bindings.yaml` in, or drop both. This does not change what the implementor does.
   - Line citations: `git grep -nE 'er\.rs:[0-9]+' 42f599466 -- ':!.engineering'` finds four `er.rs:` line citations, plus the unrelated `process.rs:248` and `server.rs:237` in a doc comment and `server.rs:57-70` in `semantics.md`. The four are `cli.yaml:2218` and `auth_bindings.yaml:521` (both in N7), and `oauth2_refresh_adversary_pass2.rs:629` (named and excluded: no test reads it). A fourth, the `er.rs:4245-4258` text itself, is the auth_bindings line. Nothing is dropped without a word.

3. Note. N6 item 13 and the parent's description of N6 ("thirteen items … left open in the tests of S9d and S9e"). Item 13 is about whether two tests keep their race after S9d, not about time. The parent promises time (120 s, 60 s per test), not race coverage.
   - It is traceable. S9d paragraph (e) states the risk, and the parent already keeps the tests the set weakens or removes in view (S8a drops a test that cannot fail).
   - A story from item 13 would claim no time, like N1, N2 and N7. So it belongs in the set as a question and need not be a story of its own.
   - The parent's one-line description of N6 should say that one item is about test validity, not cost. The title "Where do the 82 s go" does not cover item 13. N6's Defect already says so in its text.

4. Note. The parent, the heading "Claimed by a change whose estimate is not confirmed, 12 tests". The same parent says "every 'after' is an estimate and none was timed", and its own sum says 268.6 s rest on an estimated change. The heading reads as if the other rows were confirmed. Make the heading name the two kinds it lists: carried over from another machine, and cause not confirmed. This does not change a sum.

5. Note. N5 question 5 against the parent's sentence "the share of S9d's two costs in the tests under 5 s". Question 5 reads the difference of the sums of durations of the tests under 5 s in two forge jobs, with a spread rule. That is what S9d takes from the tests, not the share of REG and PARSE2. The parent's wording is looser, and the two are close enough for scope. Whether a difference between two jobs can show anything at that noise (the two forge runs differ by 220 s) is the acceptance critic's.

## Checked and found correct

- N5 question 5 and the parent's sentence: the parent hands it over (`depends_on` to S9d is in N5's frontmatter), and N5 acceptance 1 says the story is done only when all five answers are written.
- Parent count: `grep -l 'decomposes: story:the-gates-test-step-ends-within-120-s-on-the-forge'` over the story files at 42f599466 gives 23. Without the archived story that is 22: 15 change (S1 to S8c, S9b, S9c, S9d, S9e), 3 build (N1, N2, N7), 4 questions (N3 to N6) = 22.
- Parent sums recompute: before 1,117.8, after 361.2, tests 81, 368.6, 268.6 (368.6 − 49.4 − 50.6), 49.4 = 10.2 + 7.4 + 16.5 + 8.3 + 7.0, and S9d's 31.96 saving gives 108.0 − 32.0 = 76.0.
- The parent paragraph on the two open findings names S1 (paragraph on `metadata.rs`) and N2 (paragraph on the setup script), the reason (their own open pull requests) and the later correction. It matches the "Known open" list.
- N6 item 13 line citations against the source: `issue.rs` line 299 and 311 `Barrier`, line 319 the 3 ms sleep per caller; `http_standing_grant_adversary.rs` 258, 272 to 287; `http_standing_grant.rs` 957 to 984. They match.
- N6 arithmetic: 117.685 − 35.979 = 81.706; 1.0 s / 117.685 = 0.85 %; the 13.67 ms (10.63 + 3.04).
- N7 against the source: `removal_command` is at line 5093 with the arm `("connectors.cli.ConnectionListCursor", "Active")` at lines 5095 to 5097, and `update_command` at line 4910. Both start with `fn <name>(`. No test other than `the_er_rs_citation_lands_on_the_cursor_expiry_path` reads `er.rs` by path.
- The `store-tests…` closing paragraph: "eight tests named here" is right (1, 2, 3, 4, 5, 6, 7 and 9 of S9d's table), plus the two `http_standing_grant` tests. S9d's test 8 is not named in that story.

## Not checked

- `aep plan artifact waves` output (the wave numbers 1, 4, 6, 7). The brief marks N7's wave as decided, and the tool was not run.
- The forge job figures (138270416028). Only the build-host records were available.
- The dependency's source and the `mysql_async` lines. They are the parallel-safety and acceptance critics' to check.

## For another critic

- N5 question 5 compares sums of two forge jobs whose test steps varied by 220 s. Whether that can show a change at all belongs to the acceptance critic.
- N6 item 12 mixes thread-seconds and wall time, and about 9.7 s of the 21.92 s lie under no timer. The first step it names, a timer tree, will show that.
- `store-tests…` acceptance statement 2 still offers "a shared fixture built once per run" while the closing paragraph says that route is closed for these tests. That is the design critic's.
