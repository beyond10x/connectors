---
format: aep.planning-md/3
id: review-result:released-lowering-adversary-pass-2
kind: review-result
status: active
title: Adversary read of the second commit of the committed lowering
relations:
- reviews: story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition
revision: 1
---
Verdict: needs-change, for the second commit of the change of
story:the-catalog-provider-tests-read-a-committed-lowering-of-the-released-definition. No
behaviour defect that anybody reaches was found. Six behaviours the change claims were held by no
gate test: each could be removed and all 12 gate tests stayed green. The third commit of the same
pull request adds a test for each and settles four small findings; one is not changed and says
why.

Second adversary read, of the committed change. The adversary wrote 16 cases on the committed
code, 6 cases that each fail for one mutant, and 8 copies of the module with one line changed
each, and ran them on the build host of the shared pool in two runs: 178 tests run, 168 passed,
10 failed (1 on the code as committed, 9 inside the mutated copies). The 12 committed gate tests
passed. It ran no `slow_lane_` test and left the tree as it found it.

## Findings

All are in `adapters/catalog/tests/released_lowering.rs`.

1. Introduced by the second commit, shown by a mutant: no gate test failed when `check_directory`
   stopped calling `check`, because every test of `check_directory` passed the record itself as
   the current inputs.
   Outcome: fixed. A test gives `check_directory` inputs that differ from the record.
2. Introduced by the second commit, shown by a mutant: the comparison of the record's format was
   held by no test.
   Outcome: fixed. A test holds a record of another format.
3. Introduced by the second commit, shown by a mutant: comparing the input rows in every run was
   held by no test; the first commit's behaviour (rows on a digest mismatch alone) passed all 12.
   Outcome: fixed. A test holds a changed row under the recorded digest.
4. Introduced by the second commit, shown by a mutant: that a name starting with `.` inside the
   committed directory fails the fast check was held by no test.
   Outcome: fixed. A test holds it.
5. Introduced by the second commit, shown by a mutant: the `CI` values of the test of `mode` were
   `true`, `1` and empty; a `mode` that lets `CI=false` write passed.
   Outcome: fixed. The values `false` and `0` are in the list.
6. Introduced by the first commit, shown by a mutant: the extension filter of an input root had no
   test.
   Outcome: fixed. A test holds that an extension root takes its files at any depth and no other.
7. Introduced by the second commit, shown by a failing case: a link that points nowhere inside
   the committed directory panics with its path and without the regeneration command.
   Outcome: not changed. Nothing reaches it but a link committed by hand. The comment of
   `check_directory` no longer promises the command for that case.
8. Introduced by the second commit, shown by a case: the module comment said the directory is
   never written by hand, yet a lowered file edited together with its row in `inputs.json` passes
   the fast check and the engine build. A record of hashes cannot notice that; the lane test,
   which lowers again and compares every byte, does.
   Outcome: the comment says what the fast check binds and what the lane test alone sees.
9. Introduced by the second commit, shown by a case: of two rows with one path the later one
   counted, so a wrong row before the right one passed.
   Outcome: fixed. A list with a repeated path fails and names the path.
10. Introduced by the second commit, shown by a case: a backup file under the definitions root
    whose name does not start with `.` (`github.yaml~`, `github.yaml.orig`) is an input; a
    regeneration records it and the forge then fails with a removed input.
    Outcome: the stale text for an added input says to remove a file that is not to be committed.
11. Introduced by the second commit, read and not run: the failure for a digest that is not the
    one of its rows printed the current digest, which pasted into the record passes the check.
    Outcome: fixed. The text no longer holds the value.

## Attacked and not broken

- Findings 2, 4, 5 and 6 of the first read are closed: its cases are green against this head.
- An empty or absent `lowered` list fails and names the files. A row with `..`, `./` or an
  absolute path reads as missing plus not in the record; the check opens no path taken from a
  row. Swapped hashes report both rows.
- `mode` refuses `1` with surrounding whitespace; an empty `CI` and `CI=false` count as set.
- The lane on the forge compares: its workflow sets no write variable and the forge sets
  `CI=true`.
- No file or directory under the two input roots starts with `.`, in the tree or in the index.
- The body of `engine()` and the five lowered files are the same in both commits.
- One fresh lowering into a temporary directory equals the committed directory byte for byte,
  `inputs.json` included, and passes the fast check.

## Not established

- macOS: nothing ran there, and no workflow of the repository runs a test of
  `connectors-catalog-provider` on macOS.
- The lane test's body in comparison mode: it has not run, and no gate test covers that its
  write and compare arms follow `mode`.
- A lowered file replaced by a link to equal bytes is not noticed as a link.
- Bytes after a change of the toolchain or of a dependency.
