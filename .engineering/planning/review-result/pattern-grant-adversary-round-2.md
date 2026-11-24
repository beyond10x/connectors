---
format: aep.planning-md/3
id: review-result:pattern-grant-adversary-round-2
kind: review-result
status: active
title: Adversary review, round 2 (after fix round 1), story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
unit: story:standing-grant-binds-a-pattern-or-a-value-set (round 2, adversary)
verdict: NEEDS-CHANGE
cases: executed 130→151 (14 mine, 7 from the other reviewer's `standing_pattern_security_review_r2.rs`), red 6 mine
origin: introduced (by the story; F1 and F3 by the fix round)
wrote-outside-worktree: `<session scratch>/pattern/r2-adversary/` only
needs-coordinator: no

**Diff:** `git --no-pager diff --stat` shows 11 files, 286 insertions, 43 deletions, none mine. My one file is untracked: `worktree pattern-grant/crates/connectors-host/tests/standing_pattern_adversary_review_r2.rs`. It is rustfmt-clean and clippy `-D warnings` exits 0.

**Suite run:** the brief's lane, 07:26:12–07:27:04 CET (after the 06:14:58 cleanup): `151 tests run: 144 passed, 7 failed`, exit 100. The 130 earlier cases all pass. The seventh red is the other reviewer's `one_checked_write_against_a_file_within_the_stated_limits_takes_under_a_second`. My binary alone at 07:25:57: 14 run, 8 passed, 6 failed. No case of mine runs over 5 s (longest 2.6 s).

**Findings** (test lines are in my file):
- **F1, warning, contract drift.** `contracts/service/compatibility.md:56`, `contracts/cli/v1alpha1/semantics.md:969` and `docs/local-approvals.md:185` say `(?i-u)` patterns "are taken". Red `a_fold_with_unicode_off_is_taken_as_the_texts_say` (:420) shows four refused: `^(?i-u)Release review of (?s:.*)$`, `^(?i-u)Release review of .*$`, `^(?i-u)DEV-[^/]+$`, `^(?i-u:dev-.+)$`. Under `-u`, `.` and a negated class match bytes. `^(?i-u:release review of )(?s:.*)$` is taken. Fix: the texts say the fold goes around the letters only.
- **F2, blocker by the suite's own invariant 6 (under 1 s).** `crates/connectors-host/src/local/standing.rs:471`. Red `one_checked_write_within_the_stated_limits_takes_under_a_second` (:1210): "the check of one write of 3000 bytes against 64 members of ^(?s:.*)a?x{2007}$ takes 2.612737858s of CPU time", 07:26 CET, test profile. A 4,000-byte value took 7.21 s (06:08 CET). Round 1's `regex::Regex` cost the same (2.57 s against 2.66 s), so the fix round did not add it. `standing_pattern_security_review.rs:206` times the file read only.
- **F3, warning, suite gap.** `standing.rs:367`. Red `a_suite_case_tells_whether_a_flag_holds_over_a_branch` (:830): "none of 105 cases tells a `(?flags)` that holds over a `|` from one that ends at it". In the scratch copy, a mutant that resets flags at `|` leaves all 123 earlier `connectors-host` cases green. It takes `^(?i-u)s|(?u)k$`, whose wrap admits U+212A.
- **F4, note, suite gap.** `standing.rs:378`. Red `a_suite_case_tells_whether_the_fold_ends_with_its_group` (:845); the mutant that keeps the fold past its group also leaves the earlier cases green. Removing `.dfa(false)` or `.backtrack(false)` (`standing.rs:457-458`) leaves them green too. My figures case kills the first; nothing kills the second. Both mutants hold less memory. The mutants without `.onepass(false)`, `hybrid_cache_capacity` or `WhichCaptures::Implicit` are killed by the suite.
- **F5, warning.** `compatibility.md:56` names no host for its memory figures. Each pattern holds 64 bytes per CPU the process may run on (`regex-automata` pool). Measured for `^\ba?x{2034}\b$`: 333,441 bytes on 16 CPUs, 336,513 on the 64-CPU build host (06:22–06:25 CET). Red `one_read_holds_the_stated_figure_whatever_the_hosts_cpu_count` (:1075): "holds 21278846 bytes in a process that may run on 1 CPU; the contract states 21536886". By arithmetic, not measured: `standing_pattern_security_review.rs:152` (band 262,144) fails above 128 CPUs.
- **F6, note.** Red `one_read_holds_no_more_than_the_contract_states_under_a_leading_any_run` (:1184): 64 members of `^(?s:.*)(?:a?x{2004}|b)$` hold 21,692,726 bytes against 21,536,894 for "the costliest pattern found". That is 0.72 % over and inside the suite's band.
- **Notes, no test.**
  - The texts do not say that Unicode turned back on under a fold is refused (`^(?i-u)de(?u)v$`).
  - `website/docs/reference/contracts/compatibility.md` still holds 27,278,992 and no `(?i-u)`; generators were not run.
  - The fixture's "about" text calls the host rule a copy without naming the `(?i)` difference.

**Step 1, `standing_pattern_adversary_review.rs`:**
- `a_case_insensitive_pattern_admits_no_unicode_look_alike` (:322): holds. It now asserts both `(?i)` patterns are refused and keeps the look-alike assertions on `(?i-u)`; it goes red again if `(?i)` is taken (decision a).
- `a_dot_crosses_no_line_break` (:383): holds as a pin of the corrected sentence (decision b). Its name now says the opposite of what it asserts, and it does not read the text.
- `type Case` (:504): no assertion changed.

**Could not break:**
- Fold refusal: 2,500 generated patterns, none admits U+017F or U+212A; the verdict equals a second reading of the flags.
- Reader against `grant-standing`: same verdict on every generated pattern and on a 37-entry edge table (includes 256 and 257 bytes).
- Matcher against `regex::Regex` on the wrap: 3,000 generated patterns, about 80 values each, no difference.
- Memory: 9 files where the PikeVM runs after the lazy DFA gives up all hold less than the costliest; the five stated figures are met within 2 % once CPU count is taken off.
- `Cargo.lock`: `cargo metadata --locked --offline` exits 0, and cargo rewrote the pre-edit lock to a byte-identical file.

**My own corrections:**
- Three wrong expectations in my first run (06:03 CET) were fixed.
- A first form of the F5 case was red over 8 bytes of first-call noise and was rewritten.
- A case assuming a non-ASCII value holds no more than an ASCII one was false on the tree (15,503,862 against 12,898,550, both below the stated figure) and was dropped.

**Not checked:**
- Release-profile timings.
- The served HTTP path.
- The digest in `crates/connectors-build/generated/metadata-emissions/inputs.json` (acceptance 23, outside the lane).
- Step 1 for the security review file.
- A host above 64 CPUs.

**Outside the worktree:** everything is under the scratch directory above: logs, probe inputs, `copy/` (tree copy with the mutant `standing.rs`), `copy-target/`, `standing.mutant.rs`, `r2.*.rs`. Nothing was built into the checkout's `target`. The worktree lease is released.

```findings
- file: contracts/service/compatibility.md
  line: 56
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Texts say (?i-u) patterns are taken; ^(?i-u)Release review of (?s:.*)$ and three more are refused (a_fold_with_unicode_off_is_taken_as_the_texts_say).
- file: crates/connectors-host/src/local/standing.rs
  line: 471
  category: resource-exhaustion
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: One checked write of 3,000 bytes against 64 members of ^(?s:.*)a?x{2007}$ takes 2.61 s CPU; the suite times the file read only.
- file: crates/connectors-host/src/local/standing.rs
  line: 367
  category: test-gap
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: No suite case tells a flag that holds over | from one that ends there; the mutant takes ^(?i-u)s|(?u)k$, which admits U+212A, and the suite stays green.
- file: crates/connectors-host/src/local/standing.rs
  line: 378
  category: test-gap
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: No suite case pins the fold ending with its group, nor .dfa(false) and .backtrack(false) at 457-458; those mutants leave the suite green.
- file: contracts/service/compatibility.md
  line: 56
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: Memory figures hold for 64 CPUs only (64 bytes per pattern and CPU); 21,278,846 on 1 CPU against 21,536,886 stated; the suite's band case at standing_pattern_security_review.rs:152 fails above 128 CPUs (computed).
- file: contracts/service/compatibility.md
  line: 56
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 64 members of ^(?s:.*)(?:a?x{2004}|b)$ hold 21,692,726 bytes, 0.72 % over the pattern named as the costliest found.
- file: crates/connectors-host/tests/standing_pattern_adversary_review.rs
  line: 383
  category: test-quality
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a_dot_crosses_no_line_break now asserts that . crosses every break but U+000A; the assertion holds, the name says the opposite.
```
