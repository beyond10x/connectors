---
format: aep.planning-md/3
id: review-result:wave-a-bundle-index-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on bundle-index-refuses-ambiguous-rows (wave A)
relations:
- reviews: story:bundle-index-refuses-ambiguous-rows
revision: 1
---
```
unit: story:bundle-index-refuses-ambiguous-rows
verdict: green
cases: executed 407→408, red 1
origin: introduced 0, pre-existing 0, undecided 1
wrote-outside-worktree: <scratch>/units/bundle-index-refuses-ambiguous-rows/scratch/{adv-case.log,adv-suite.log,adv-suite-nff.log}
needs-coordinator: no
```

The Acceptance holds. `read_index` refuses all three scratch shapes with `InvalidInput`: two rows for one provider, an absolute `file_name`, and a `../` `file_name`. One case I added is red. It breaks the story's Outcome line ("`load` never reads a file outside the bundle directory"), not its Acceptance. This covers commit 1267778935 plus my one untracked test file.

1. **Diff:** `git diff --stat` is empty. The only change is the untracked test file `crates/connectors-catalog/tests/bundle_adversary.rs`. I touched no implementation file.

2. **Added case:** `a_bundle_file_that_links_outside_the_directory_is_not_followed`, unix only.
   - **Setup:** `write` makes a normal index, then `fixture.bundle.json` is replaced by a symlink to the same bundle in a sibling directory. The rows pass every check, and the case asserts that.
   - **Assertion:** `load(&directory, "fixture")` must be `Err`.
   - **It is red now.** Command: `cargo test --locked -p connectors-catalog --test bundle_adversary`. Output: `panicked at crates/connectors-catalog/tests/bundle_adversary.rs:46:5: load read …/elsewhere/fixture.bundle.json through a link out of …/bundles`, then `test result: FAILED. 0 passed; 1 failed`, EXIT=101.
   - **What reaches it:** only a hand-built bundle directory, the same reach the story rates Low. `bundle_directory` in the local catalog config goes to `bundle::load` (`adapters/catalog/src/local.rs:1742`, `:1884`), and `write` never creates a link.
   - **Origin:** the base `load` has the same join-and-read code and no row checks, but I did not run it against the base, so it stays undecided.
   - **Fix (not applied):** have `load` check `std::fs::symlink_metadata(&path)` and refuse anything that is not a regular file.

3. **Package suite:**
   - `cargo test --locked -p connectors-catalog` stopped at the first failing target (EXIT=101).
   - Rerun with `--no-fail-fast`: 407 passed and 1 failed (the case above), EXIT=101. The 407 matches the implementor's count.
   - `rustfmt --check` on the new file: exit 0.

4. **Judgement, not blocking:** `shipped_bundle` (`adapters/catalog/src/local.rs:467-470`) says it checks the compiled-in index "as `bundle::load` checks a directory". It still calls `index.find` without `check_rows`, so that comment now promises more than the code does. Nothing reaches it today: the shipped index is generated, its 16 rows are all `<provider>.bundle.json`, and none repeats. This is a doc-only note, so I left it out of the findings block.

5. **Attacked and could not break:**
   - Each of the three checks has a test that goes red if the check is removed (one per check, checked by reasoning against the diff).
   - `valid_id` allows `.`, so I tried the providers `.`, `..` and `.index.writing`. None of them makes a file name with a `/` or one that clashes with the index or lock file.
   - `write` already refuses invalid ids, so it cannot produce an index that `read_index` rejects.
   - The committed `adapters/catalog/generated/bundles/index.json` passes the new rule.
   - No other package builds an index by hand.

```findings
[{"file": "crates/connectors-catalog/src/bundle.rs", "line": 342, "category": "acceptance", "severity": "warning", "verdict": "CONFIRMED", "origin": "undecided", "message": "load follows a symlinked <provider>.bundle.json out of the bundle directory, so the story's Outcome does not hold; red case at crates/connectors-catalog/tests/bundle_adversary.rs:46"}]
```
