---
format: aep.planning-md/3
id: review-result:pattern-grant-security-round-2
kind: review-result
status: active
title: Security review, round 2 (after fix round 1), story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
unit: story:standing-grant-binds-a-pattern-or-a-value-set, round 2, role `security`
verdict: NEEDS-CHANGE
cases: executed 130→149, red 5 (1 mine; 4 in the other reviewer's `standing_pattern_adversary_review_r2.rs`; 130 is the brief's figure, not run by me)
origin: introduced 5 / pre-existing 1 / undecided 0
wrote-outside-worktree: yes, scratch only (section 6)
needs-coordinator: yes; decide whether the check of a write gets a CPU bound (F1), and run the docs generator (F2)

**1. `git --no-pager diff --stat`** (unstaged): 11 files changed, 286 insertions(+), 43 deletions(-), all the fix round's. Mine is one untracked file, `crates/connectors-host/tests/standing_pattern_security_review_r2.rs`. Staged is unchanged at 57 files.

**2. Cases added** (7; each run alone before the lane; clippy `-p connectors-host --tests -- -D warnings` exit 0 at 06:13:03 CET)
- RED `one_checked_write_against_a_file_within_the_stated_limits_takes_under_a_second`: `FAIL [ 4.082s]` alone at 06:08:30 CET, exit 100; `FAIL [ 6.717s]` in the lane (over 5 s). Message: "the check of one write of a 4000-byte value against a 10871-byte file with 64 members of ^x*x{2000}$ takes 6.663782955s of CPU time (the read before it 22.135419ms)", panicked at `standing_pattern_security_review_r2.rs:858:5`.
- Green, none over 5 s:
  - `a_generated_pattern_is_taken_by_the_stated_rule_and_never_folds_beyond_ascii` (1.4 s; 3,000 seeds, 1,525 taken, 1,045 refused by the rule)
  - `the_named_forms_of_a_case_fold_are_refused_or_fold_ascii_only` (32 refused, 14 taken)
  - `a_fold_written_as_a_flag_turns_unicode_off_for_the_rest_of_the_pattern` (`PASS [ 0.007s]`, 06:11:45 CET)
  - `the_matcher_answers_as_the_regex_crate_does_on_the_wrapped_pattern` (18,150 answers compared)
  - `the_other_measured_figures_of_the_contract_hold`
  - `the_documented_patterns_check_a_request_sized_value_within_a_second`

**3. Suite**: `CONNECTORS_ESS=<ess 0.97.4> cargo nextest run --locked --no-fail-fast -p connectors-host -p connectors -p connectors-catalog -E 'test(standing) | test(whole_value_patterns) | binary(~standing_grant) | binary(~standing_pattern)'`, 06:11:56 to 06:12:57 CET.
- Result: `Summary [ 13.930s] 149 tests run: 144 passed, 5 failed, 2044 skipped`, exit 100. Slowest test 12.8 s.
- The four other reds, all in `standing_pattern_adversary_review_r2`:
  - `a_suite_case_tells_whether_a_flag_holds_over_a_branch`
  - `a_suite_case_tells_whether_the_fold_ends_with_its_group`
  - `one_read_holds_no_more_than_the_contract_states_under_a_leading_any_run`: 64 × `^(?s:.*)(?:a?x{2004}|b)$` hold 21,692,734 bytes; the contract states 21,536,886
  - `one_checked_write_within_the_stated_limits_takes_under_a_second`: 4.75 s CPU, the same defect as F1

**4. Findings**

| # | Severity | file:line | Verdict | Origin | Measured | What reaches it |
|---|---|---|---|---|---|---|
| F1 | warning | `crates/connectors-host/src/local/standing.rs:472` | NEEDS-CHANGE | introduced by the story; pre-fix matcher not measured | Scratch probe: `^x*x{2037}$` on a 60,000-byte value takes 1.004 s per pattern at opt-level 1 (06:06:57 CET) and 0.796 s in release (06:07:39 CET). That is 50 to 64 s for 64 members. | The signer writes the pattern, the caller picks the value (64 KiB served, 1 MiB local, up to three reads per write). No text states a bound for the check; acceptance 25 bounds the parse only. |
| F2 | warning | `website/docs/reference/contracts/compatibility.md:71`, `website/docs/reference/contracts/cli.md:986` | NEEDS-CHANGE | introduced by the fix round | Both still state "26 MiB" (`:71` also 426,202 and 27,278,992 bytes). No file under `website/docs` holds `(?i-u)`. | Generated copies not regenerated; I ran no generator. |
| F3 | note | `contracts/cli/v1alpha1/semantics.md:969` (same sentence in the other four texts) | NEEDS-CHANGE | introduced | The texts say `(?i-u)` is taken. The reader refuses `^(?i-u)dev-.*$`, `^(?i-u)dev-[^ ]+$` and `^(?i-u)dev-\W$` (as the regex crate does), and `^(?i-u)dev(?u)-.*$` (by the rule). Behind the flag `\d` and `\w` are ASCII only. Only `(?i-u:...)` leaves the rest Unicode. | A signer following the texts' example. |
| F4 | note | `crates/connectors-host/tests/fixtures/whole_value_patterns.json:2` | NEEDS-CHANGE | introduced | "about" still calls the host rule a copy that differs by size limit only; the host now also refuses `(?i)`. | Readers of the fixture. |
| F5 | note | `crates/connectors-build/generated/metadata-emissions/inputs.json:133` | NEEDS-CHANGE | pre-existing at HEAD | Recorded sha256 `47772647…` for `ess/domains/standing_approvals.yaml`; the working file is `42edb23f…`. | Acceptance 23's test `the_committed_metadata_emissions_are_current`, not run (outside the lane). |
| F6 | note | `crates/connectors-core/generated/operation-types/` (3 staged files) | NEEDS-CHANGE | introduced | Acceptance 21a says no file under `crates/connectors-core` changes. | The literal acceptance check. |

Step 1, `standing_pattern_security_review.rs`, four changed assertions, judged by reading (no mutant run):
- `STATED_ONE_READ` 27,278,992 → 21,536,886: holds. Decision (c), and the constant is lower, so stricter.
- `the_costliest_pattern_of_the_contract_holds_what_the_contract_states`: holds. It follows the contract's named pattern, the band is tightened to ±262,144, and the old pattern is still asserted ≤ stated.
- `one_read_of_a_file_within_the_stated_limits_holds_no_more_than_the_contract_states`: holds. Assertion unchanged.
- `one_read_of_a_file_within_the_stated_limits_takes_under_a_second`: holds. Decision (a): the `(?i)` file must now be refused within 1 s, and four accepted costly patterns must each parse within 1 s.

**5. Reviewed and could not fault**
- Invariant 2: no taken pattern, generated or named, matched a value with U+212A or U+017F. The conservative forms `^(?i)(?-u)k$`, `^k(?i)$`, `^((?i))k$` and `^(?i:)k$` are refused.
- Invariant 3: `check_request` and `parse` agreed on every pattern tried.
- Invariant 4: every answer equals `regex::Regex` on `\A(?:pattern)\z`.
- Invariant 5: `cargo metadata --locked --offline` exit 0 at 05:56:13 CET, `Cargo.lock` sha256 unchanged.
- Parse CPU: at most 161 ms for 64 builds (`^[\PC~~\pL×49]$`).
- Memory: the stated figures reproduce (6,579,638; 1,670,774; 927,670; one member 336,521 against 336,513 stated). My search found nothing above 332,087 per pattern. The other reviewer's case does, by 0.72 %, so "the costliest pattern found" in the texts is out of date.

Not checked: the adversary review files; mutants against the edited round 1 tests; `the_committed_metadata_emissions_are_current` and the generators; workspace clippy; peak memory during a read; check CPU through the local three-reads path; Windows.

**6. Written outside the worktree**, all under `<session scratch>/pattern/r2-security/`:
- `probe/Cargo.toml`, `probe/Cargo.lock`, `probe/src/main.rs`, `probe/target/`
- `memory1.log`, `quit1.log`, `time1.log`, `search1.log`
- `matching4000.log`, `matching60000.log`, `matching60000-release.log`
- `lock.before.sha256`, `metadata.json`, `metadata.err`
- `red-alone.log`, `own-others.log`, `scope-alone.log`, `lane.log`, `clippy.log`

My lease is released (`worktree hook session-end`, exit 0).

**7.**
```findings
- {file: crates/connectors-host/src/local/standing.rs, line: 472, category: denial-of-service, severity: warning, verdict: NEEDS-CHANGE, origin: introduced, message: "The check of one write has no CPU bound: 64 members of ^x*x{2000}$ against a 4000-byte value take 4.1 to 6.7 s; ^x*x{2037}$ against 60,000 bytes takes 1.0 s per pattern."}
- {file: website/docs/reference/contracts/compatibility.md, line: 71, category: stale-generated-text, severity: warning, verdict: NEEDS-CHANGE, origin: introduced, message: "Generated website copies still state 426,202 / 27,278,992 bytes (26 MiB; also cli.md:986) and hold no (?i-u) rule."}
- {file: contracts/cli/v1alpha1/semantics.md, line: 969, category: text-versus-code, severity: note, verdict: NEEDS-CHANGE, origin: introduced, message: "Texts say (?i-u) is taken; behind the flag '.', negated classes and (?u) are refused and \\d, \\w are ASCII only. Only (?i-u:...) keeps the rest Unicode."}
- {file: crates/connectors-host/tests/fixtures/whole_value_patterns.json, line: 2, category: text-versus-code, severity: note, verdict: NEEDS-CHANGE, origin: introduced, message: "'about' names only the size-limit difference; the host rule also refuses (?i) with Unicode on."}
- {file: crates/connectors-build/generated/metadata-emissions/inputs.json, line: 133, category: stale-generated-text, severity: note, verdict: NEEDS-CHANGE, origin: pre-existing, message: "Recorded digest of ess/domains/standing_approvals.yaml differs from the working file; acceptance 23's test not run."}
- {file: crates/connectors-core/generated/operation-types/types.rs, line: 1, category: acceptance, severity: note, verdict: NEEDS-CHANGE, origin: introduced, message: "Acceptance 21a: three staged files under crates/connectors-core changed."}
```
