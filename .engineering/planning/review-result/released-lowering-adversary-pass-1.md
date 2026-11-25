---
format: aep.planning-md/3
id: review-result:released-lowering-adversary-pass-1
kind: review-result
status: active
title: Adversary read of the first commit of the committed lowering
relations:
- reviews: story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition
revision: 1
---
Verdict: needs-change, for the first commit of the change of
story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition. Four
introduced findings are fixed in the second commit of the same pull request; one is the merge
order the story names; one is not changed and says why.

Adversary read of the committed change. The adversary added 12 cases in a module of its own,
uncommitted, and ran them on the build host of the shared pool: 16 tests run with the change's
four gate tests, 12 passed, 4 failed (its own). It edited no file of the change and ran neither
the `slow_lane_` test nor `connectors-build ignored`. One of its cases lowered the released
definition once into a temporary directory and compared: the fresh lowering equals the six
committed files byte for byte.

## Findings

1. Introduced, read and not run: at the first commit the new ignored test
   `released_lowering::slow_lane_the_committed_lowering_equals_a_fresh_lowering` is unknown to the
   gate's ignored step, because the rule that puts a `slow_lane_` test into the timing family is
   the change of story:the-lane-slow-runs-the-ignored-timing-family.
   Outcome: the merge order the story names (`depends_on`). The pull request opens after that
   story is on main.
2. Introduced, shown by three failing cases: the fast check bound the inputs only. A committed
   directory whose `github/connector.json`, `github/operations.json`, or bundle together with its
   row in `index.json` was edited by hand passed the fast check and the engine build. Nothing
   reaches it but a hand edit; an edit of the bundle alone fails in `bundle::load`.
   Outcome: fixed. `inputs.json` records one sha256 per lowered file and the fast check recomputes
   them.
3. Introduced, read and not run: the digest does not cover
   `crates/connectors-catalog/Cargo.toml`. The adversary also named the root `Cargo.toml` and
   `rust-toolchain.toml` as files that start neither the gate's check nor the lane.
   Outcome: not changed. The story keeps `Cargo.lock` out of the digest for the reason that
   holds for these three files too: the lane test lowers again and compares every byte, and the
   lane's two path lists in the change of story:the-lane-slow-runs-the-ignored-timing-family hold
   `crates/**`, `Cargo.toml` and `rust-toolchain.toml`. The adversary read the path lists of the
   base, where the three are missing.
4. Introduced, read and not run: write mode was on for any value of
   `CONNECTORS_WRITE_RELEASED_LOWERING`, also an empty one and `0`, and then returned without
   comparing. Set in the lane's environment by mistake the lane test would pass on a stale
   directory. Nothing sets it.
   Outcome: fixed. The test writes for the value `1` alone, refuses any other set value, and
   refuses a set variable when `CI` is set.
5. Introduced, shown by one failing case: a dangling link named `*.rs` under an input root
   panicked without its path; the definitions root counted every file, so an untracked
   `.DS_Store` read as an added input.
   Outcome: fixed. A failed read names its path; a name that starts with `.` is not an input.
6. Introduced, read and not run: an `inputs.json` that does not parse failed without the
   regeneration command; a wrong digest over correct per-file rows printed an empty list of
   changed files.
   Outcome: fixed. Both failures say what is wrong and print the command.

## Attacked and not broken

- The three old `engine()` bodies were byte-identical; the new function loads the same bundle
  through `bundle::load` with the same second argument and the selections of the same file.
- The comparison function names a file that is missing, one that is extra and one that is cut
  short.
- The digest depends on a file's path and content; the extension filter holds at any depth.
- The regeneration command is the same, character for character, in the story, the constant, the
  test's literal and `inputs.json`.
- All 24 fixture and input files are LF or binary in the index and in the tree; no file holds a
  CR.
- No `HashMap` is on the output path; `serde_json` has no `preserve_order` in the workspace.

## Not established

- The bytes of a lowering on macOS and on the forge's runner.
- Whether a change of the toolchain or of a dependency feature alters the bytes.
