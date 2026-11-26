---
format: aep.planning-md/3
id: review-result:replan-template-store-design-round-3
kind: review-result
status: active
title: Design critic, round 3, the re-plan of the template-store story
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: **approve**

## Round-2 findings of the design role

1. N7, red run of the existing test: resolved. The Defect now states the tolerance (1 or 2 lines added, or 1 removed, stays green; 3 added or 2 removed goes red). Acceptance 1 inserts 10 lines. The shift was redone by reading `er.rs`: the arm is line 5095, the `Expire…` string 5096, the closing brace 5097. A shift of +1, +2 or −1 keeps line 5095 or 5096 in the window. A shift of +3 puts it on lines 5093 and 5094 (the `fn` line and the `let command` line), which do not hold `ConnectionListCursor`, so the test fails. A shift of +10 fails the same way. With N7 the finder takes text, so the same 10 lines pass.
2. S9d paragraph (e) against N6 item 13: resolved. Both name the same four tests (2, 6, 8, 10) and both say no overlap is shown. The line numbers match the source: `mutations/tests.rs` has the `Barrier` at line 497, `assert_ne!(gate.is_ok(), abort.is_ok())` at 509, and `owner/refresh/tests/issue.rs` has the `Barrier` at 311. Test 11 is kept apart as the one that shows its overlap. Item 13 now has an entry for test 8.
3. Nine callers of `definition_bundle()` in the test module: resolved. S9d line 129 lists the nine lines. `grep` finds exactly those nine, plus three above line 5095 outside the test module: 993, 1170 and 4328.
4. N6 item 12 against item 8 and acceptance 1: resolved. Item 8 owns `er.start`, `resynchronize` and `complete_snapshot` for all eleven tests. Item 12 states it gets no entry for them. Acceptance 1 gives item 12 one entry per timer over 1.0 s of own time plus one for the rest.
5. 27 paths in `inputs.json`: resolved. `grep -c 'ess/domains/'` on the file gives 27, and `git ls-files ess/domains` also lists 27.
6. N7 claims as exact arm text and the citation form: resolved. Change 3 pins both claims as exact text.
   - `removal_command` (line 5093 to the `}` at 5106): exactly one line holds `"connectors.cli.ConnectionListCursor"`. It is line 5095, whose trimmed text is `("connectors.cli.ConnectionListCursor", "Active") => {`. The next line holds `"connectors.cli.ExpireConnectionListCursor"`.
   - `update_command` (line 4910 to the `}` at 5062): `"connectors.auth_bindings.PublishBinding"` appears once, at line 4934. The line before it is `("connectors.auth_bindings.Connection", ("Live" | "Stale", "Live")) => {`. The only other hit, at line 5173, is outside the body.
   - `fn removal_command(` and `fn update_command(` each occur once at column 0.
   - `git grep` for `er.rs:<digits>` under `ess/` finds exactly the two comments the story names, and nothing under `ess/` ends in `owner.rs:` or `server.rs:`.
   - The text now says only the colon form is checked.

## Blocking findings

None.

## Notes

1. N7 Change 1 and 3 mix two forms. `execution_audit.yaml` writes metadata/er.rs `command_arguments`, with no backticks around `er.rs`. The N7 text says "`er.rs` and `removal_command`". Pin the exact string the test matches before writing the comment, and keep it on one comment line, because a wrap between the file name and the function name would break a literal match.
2. Acceptance 1 says "at the base commit" without naming it. State that it is the head the pull request branches from (f9f263a2 today).
3. N6 has `depends_on` S9d, while item 13 is counted on S9d's pull-request head, before the merge. The text says so, and it is treated as decided. The tool will place N6 after S9d's merge, so the item-13 timing lives only in the text.
4. N6 item 12 asks for the timers of run 5 as a tree. The tree will contain the three timers item 8 owns. The text excludes them from item 12's entries, so keep that exclusion when the tree is read.

## Checked and found correct

- Edges. `aep plan artifact graph` gives N7 no `depends_on`, S9d `depends_on` N7, N6 and N5 `depends_on` S9d, and the parent `depends_on` N5. The parent also has `depends_on` edges to its other children. `store-tests-end-within-60s-under-load` `depends_on` S9d. S9d and S9e both `supersedes` the archived story. `aep plan artifact validate` prints `valid`. There is no cycle.
- Waves. `aep plan artifact waves --kind story --status draft` gives N7 wave 4, S9d wave 6 and the parent wave 7. S9e is in wave 1.
- Surface ownership. N7 reads `er.rs` and S9d writes it, but S9d changes neither `removal_command` nor `update_command`. No `fn` of those names exists at column 0 in the test module, so S9d's added tests cannot confuse the finder.
- S9e touches only `adapters/sql/tests/mysql/fixture.rs`, and no other story in the set owns that file.
- The five commits that moved the numbers exist. The first three change `cli.yaml` and the test. The last two change `cli.yaml` and `inputs.json` only.
- `ess/` holds 29 tracked files, all YAML, so the scan for `er.rs:<digits>` has a small, fixed surface.
- N6 item 13: the test names, files and line ranges match the source. The fifth multithreaded test, `replays_waiting…`, is excluded for a reason shown in the source.

## Not checked

- No test, build or timing ran. The claim that a comment-only change leaves the emitted manifests unchanged remains a reading. It stays outside this round as decided.
- The ranges of item 13 (0 to 120, 0 to 8, 0 to 24, 0 to 42) are arithmetic on the loop counts, which were not checked against the source.
- The `lock_within` counter for tests 6 and 8, and where test 8's two threads meet. The draft says these were not read.

## For another critic

- Parallel safety: `ess/domains/cli.yaml` and `inputs.json` are shared with several drafts that record no scope. N7's scope section lists them as unassessed, which is the open point for that role.
