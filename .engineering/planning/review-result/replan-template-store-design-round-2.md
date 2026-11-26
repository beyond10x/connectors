---
format: aep.planning-md/3
id: review-result:replan-template-store-design-round-2
kind: review-result
status: active
title: Design critic, round 2, the re-plan of the template-store story
relations:
- reviews: story:the-gates-test-step-ends-within-120-s-on-the-forge
revision: 1
---
Verdict: needs-revision. Read at commit 42f599466. N7 is the citation draft, S9d the registry
draft, S9e the MySQL fixture draft, N6 the question story of the re-plan, N5 the question story
of the tests under 5 s.

## Round-1 findings

1. S9e, tenth binary: resolved.
2. N6 edge to S9d: resolved.
3. N6 item 11 against N5: resolved.
4. N6 items 2 and 6, parts answerable by reading: resolved.
5. S9d, the `registry()` callers: resolved.
6. S9d, acceptance 1 covers only the registry half: resolved in form; one count in point 5 is
   wrong (new finding 3).
7. Clone cost: resolved.
8. `simulate_process` and a warm cache: no finding.
9. Concurrent tests: resolved in part (new finding 2).
10. S9e source: no defect.
11. Stale ownership in sibling stories: resolved in part; the two open on purpose are excluded,
    the implemented ones keep their history.

## New findings

1. Blocking. N7, the Defect sentence "A change that adds or removes one line above line 5095
   makes it red" and acceptance 1 (red run with one empty line inserted). Both are false against
   the source. Line 5095 is the arm, 5096 the `Expire…` string, 5097 the closing brace. With the
   test's rule applied to the window after each shift: 1 or 2 lines inserted, green; 3 lines
   inserted, red; 1 line removed, green; 2 lines removed, red. Settles it: "at least 3 lines
   inserted at the top of `er.rs`" in acceptance 1, and the real tolerance (+2, −1) in the
   Defect.
2. Note, with a product-risk edge. S9d (e) and N6 item 13. A `Barrier` fixes the common start;
   it does not show that two callers overlap in the persist. Test 8 asserts
   `assert_ne!(gate.is_ok(), abort.is_ok())` (`mutations/tests.rs` near line 508), which also
   holds one after the other, so test 8 is in the class of tests 2 and 10. Test 2 also starts at
   a Barrier (line 311); the 3 ms sleep comes on top. Item 13 measures after S9d has merged, so
   the guard may already be weaker. Settles it: say "no overlap is shown for tests 2, 8 and 10;
   6 and 11 are not read to the same depth", and extend item 13 to test 8 or say why not.
3. Note. S9d point 5: `definition_bundle()` has nine callers in the test module (6119, 6460,
   6539, 6784, 6877, 7078, 7534, 7591, 8288), not four.
4. Note. N6 item 12 against acceptance 1: the item is at least five timers, each possibly of
   another kind. It also takes 4.01 s of `er.start`, `resynchronize` and `complete_snapshot`
   from the other ten tests while item 8 owns the same timers for one test. Settles it: one
   entry per timer over 1.0 s of own time; item 8's answer covers the other ten tests too.
5. Note. N7 Defect: `inputs.json` has 27 paths under `ess/domains/`, not 26.
6. Note. N7 Change 3. An arm `"Active" | "Expired"` would still pass; a Revoked arm that maps
   to `PublishBinding` would still pass. Settles it: require the arm pattern text
   `("connectors.cli.ConnectionListCursor", "Active")`; for `update_command` require that the
   arm yielding `PublishBinding` has no `"Revoked"` in its pattern. The form `er.rs: <name>`
   differs from forms already under `ess/domains/` (`er.rs local_approval_issuers` in
   `approval_issuers.yaml` line 15; `er.rs` with a backticked name in `execution_audit.yaml`
   lines 282 and 378): say that only the colon form is checked.

## Edges

- No cycle. N7 has no edge; S9d depends on N7; N6, N5 and
  `store-tests-end-within-60s-under-load` depend on S9d; the parent depends on N5.
- The edge S9d to N7 orders something real.
- The wave numbers in the parent reproduce.

## Checked and found correct

- `er.rs:5095-5097` is the arm of `removal_command`; `update_command` holds `PublishBinding` at
  line 4934; lines 4245 to 4258 are the read-use check. `update_command` is the function the
  auth_bindings comment means. One `fn update_command(` and one `fn removal_command(`, both
  ended by a brace in column one.
- A comment-only change leaves the emitted manifests unchanged, by reading: `inputs()` hashes
  every file under `ess/`, so `inputs.json` changes; the manifests carry no line or span.
- The three number-moving commits each change `cli.yaml` and the test.
- S9d: the callers of `registry()` outside the test module and the `timed`/`take_costs` seams.
- The citations of S9d (e) match the source.

## Not checked

- No test, build or timing was run; the emission claim is a reading.
- N6 items 1 to 8 against the dependency's internals; the parent's sums.

## For another critic

- Parallel safety: N7 and `owner-observe-request-is-modelled-in-ess` both regenerate
  `inputs.json`; the draft records "whoever merges second", without an edge.
