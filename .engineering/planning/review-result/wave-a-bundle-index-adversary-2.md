---
format: aep.planning-md/3
id: review-result:wave-a-bundle-index-adversary-2
kind: review-result
status: active
title: Adversary pass 2 on bundle-index-refuses-ambiguous-rows (wave A)
relations:
- reviews: story:bundle-index-refuses-ambiguous-rows
revision: 1
---
```
unit: story:bundle-index-refuses-ambiguous-rows (impl/bundle-index-refuses-ambiguous-rows @ 7d48775c9, plus one uncommitted test file)
verdict: red
cases: executed 12→14, red 2
origin: introduced 1, pre-existing 0, undecided 1
wrote-outside-worktree: <scratch>/units/bundle-index-refuses-ambiguous-rows/scratch/adv2-cases.log, .../scratch/adv2-suite.log
needs-coordinator: yes
```

1. **Diff.** `git diff --stat` shows nothing because the only change is one new untracked file, `?? crates/connectors-catalog/tests/bundle_adversary_2.rs`. It is a test file and is not committed. No implementation file was touched.

2. **Cases** in `crates/connectors-catalog/tests/bundle_adversary_2.rs`. Both are red now and were red on their first run alone (`cargo test --locked -p connectors-catalog --test bundle_adversary_2`, EXIT=101).
   - `an_index_that_links_outside_the_directory_is_not_followed`: `index.json` is a symlink to an index outside the directory, and `load` still returns `Ok`. Red: `panicked at ...bundle_adversary_2.rs:48:5: load read the index .../elsewhere/index.json through a link out of .../bundles`
   - `a_bundle_file_swapped_for_a_link_after_the_check_is_not_followed`: a second thread keeps swapping the bundle file between a regular file whose bytes fail the digest and a symlink to the matching bundle outside. Neither state loads on its own, so an `Ok` means the check passed on the regular file and the read then went through the link. Red: `panicked at ...bundle_adversary_2.rs:104:5: ... after checking a regular file (489 attempts)  left: 1  right: 0`. It went red on both runs. It is a race, so the attempt count will vary between runs.

3. **Suite.** `cargo test --locked -p connectors-catalog --test bundle --test bundle_adversary --test bundle_adversary_2`, EXIT=101:
   - bundle: `test result: ok. 11 passed`
   - bundle_adversary: `test result: ok. 1 passed`
   - bundle_adversary_2: `test result: FAILED. 0 passed; 2 failed`

   The before count of 12 comes from the implementor's commit message ("bundle 11 passed, bundle_adversary 1 passed").

4. **Findings.**
   - **A1** `crates/connectors-catalog/src/bundle.rs:75`. NEEDS-CHANGE, warning, origin undecided. I did not run the case against the base; `git show 881139ac3` shows `read_index` there reads the same way. `read_index` reads `index.json` through any symlink, so the story's Outcome ("load never reads a file outside the bundle directory") still fails. The F1 fix covered only the bundle file. Reach: the same hand-edited build tree that F1 needs. Acceptance holds without this fix. The Outcome does not.
   - **A2** `crates/connectors-catalog/src/bundle.rs:343`. INFEASIBLE, note, origin introduced. The F1 correction checks the path with `symlink_metadata`, then `std::fs::read` opens the same path a second time and follows a link put there in between. Reach: nothing found beyond a concurrent writer that already has write access to the directory, and that writer could rewrite the index anyway.
   - **Fix for both (I have not applied it):** open the file once with `O_NOFOLLOW` (`OpenOptionsExt::custom_flags`), check `file.metadata().is_file()` on that open handle, and read from the handle. Do the same for `index.json` in `read_index`.

5. **What did not break.**
   - A FIFO or directory at the bundle name is refused, because `is_file` is false.
   - A missing bundle file still returns NotFound.
   - Nothing in the repository ships a symlinked bundle (`git ls-files -s` lists no mode 120000), so the new refusal breaks no existing user.
   - All callers of `load` (`import.rs:833` and `:1946`, `definition.rs:3604`) behave the same as before.
   - A hard link is a name inside the directory, so it does not count as an escape.

6. **Written outside the worktree.** The two log files named in the header, both in the assigned scratch directory. Inside the worktree, `.local/tmp` is gitignored and now empty.

```findings
[{"file":"crates/connectors-catalog/src/bundle.rs","line":75,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"undecided","message":"read_index follows a symlinked index.json out of the bundle directory, so load still reads a file outside it and the story's Outcome does not hold"},{"file":"crates/connectors-catalog/src/bundle.rs","line":343,"category":"concurrency","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"the F1 correction checks symlink_metadata and then reads by path again, so a link swapped in between is followed; only a concurrent writer to the directory reaches it"}]
```
