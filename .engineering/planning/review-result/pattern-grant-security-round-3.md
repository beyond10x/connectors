---
format: aep.planning-md/3
id: review-result:pattern-grant-security-round-3
kind: review-result
status: active
title: security review, round 3 (after fix round 2), story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
unit: security review r3, story:standing-grant-binds-a-pattern-or-a-value-set (connectors 0.78.0, after fix round 2)
verdict: no blocker; 1 warning, 3 notes; I1–I7 hold on everything I ran
cases: executed 157→171, red 0
origin: introduced
wrote-outside-worktree: `<session scratch>/pattern/r3-security/` (logs only)
needs-coordinator: decide F1 (state the read figure, or bound the read); correct the peak figure in F2

T = `worktree pattern-mutants`, L = the scratch directory above.

**Tree.** `git --no-pager diff --stat`: `56 files changed, 5569 insertions(+), 2554 deletions(-)`, as before I started. My one file is untracked: `?? crates/connectors-host/tests/standing_pattern_security_review_r3.rs` (status 63 → 64 lines). No production edit; `standing.rs` mtime is 08:47:13.

**Cases alone** (`cargo nextest run --locked --no-fail-fast --no-capture -p connectors-host -E 'binary(standing_pattern_security_review_r3)'`, build-1d, 09:21:42–09:21:55 CET, nextest run 8a6b19fa, L/alone-7.log):
`Summary [   7.924s] 14 tests run: 14 passed, 0 skipped`
- The first complete run (L/alone-2.log, 09:11) had 2 red, both errors in my own cases: a 257-byte pattern, and `[\x00-\x7f--a]` listed as refused (it is taken and ASCII only). No case was ever red against the product.

**Lane** (brief's command, final state, 09:22:01–09:22:16 CET, run cf515326, L/lane-3.log):
`Summary [   8.945s] 171 tests run: 171 passed, 2043 skipped`
- Over 5 s, neither mine: `replays_waiting_on_an_original_hold…` 8.5 s, `adversary_a_concurrent_duplicate…` 8.1 s.
- Clippy exit 0 (09:21:32, L/clippy-3.log), after two clippy runs (L/clippy-1.log, L/clippy-2.log) failed on a `type_complexity` error in my file, which I fixed. `cargo fmt --package connectors-host -- --check` exit 0 (09:21:38).
- Job ids: slot printed none because no job queued; `slot status` showed lane-1 as node job 4915 and clippy-1 as 4943.

**Findings**
- **F1, warning, CONFIRMED, introduced.** `T/contracts/service/compatibility.md:56` and `T/crates/connectors-host/tests/standing_pattern_security_review.rs:255`.
  - Wrong: the costliest read is about 7 times the one the suite times, and no text gives a read figure.
  - Evidence: 64 members of `^[^\P{age=16.0}×20 a]$` (247 bytes) read in 359.7 ms CPU (L/alone-7.log:129), and 610.8 ms in one earlier run (L/alone-4.log:130). `^[\pL×82]$` took 48–52 ms in the same runs.
  - Reach: acceptance 25 (under 1 s) holds, with a margin of 1.6 to 2.8 times. The served host reads the file on every keyed write (`service.rs:460`); the local owner reads it up to three times per write (`execution.rs:166, 209`).
  - Smallest fix: state the read figure beside the check figure in § 2.1 and keep my age case. If the margin is too thin, refuse `\p{age=…}` in `Pattern::new` (`standing.rs:539`).
- **F2, note, CONFIRMED, introduced.** `T/contracts/service/compatibility.md:56`: "4,251,225 bytes … for the pattern with the highest peak found" is not the highest.
  - Evidence (L/alone-7.log, `HOLDS` lines): 4,315,312 for 64 × `^(?s:.*)[64-range class]{111}$`; 4,393,287 for 63 of the costliest plus one `[\w[\w…` nested 60 deep.
  - "No more than 5 MiB" holds, with 849,593 bytes spare. Fix: replace the figure and pattern.
- **F3, note, CONFIRMED, introduced.** The five texts name `.`, a negated class and `\W` as refused in byte mode. `\D`, `\S`, `[[:^alpha:]]` and `[^\W]` are refused too, and `^(?i-u)[\x00-\x7f--a]$` is taken (ASCII only, tested). No pattern the texts call refused is taken. Fix: one clause in the texts, or nothing.
- **F4, note, CONFIRMED, introduced; not a security defect.** `^\w{2}$` and `^\pL{2}$` are refused; the largest taken is n = 1, and 12 for `\d` (L/alone-7.log:122–125). The `regex` crate gives the same limit for `\d`. A signer who writes `^\w+-\w+$` gets a refusal. Fix: a sentence in `T/docs/local-approvals.md`.

**Could not be faulted**
- **I1:** one byte over the bound is `TooLong` for 9 match-all spellings; the bound counts bytes; a 1 MiB value is refused in under 100 ms. Served host end to end: 403 `approval_required`, same body as another target, 0 attempts, uses and provider requests; at the bound it answers 200. Logs carry the variant name or a fixed string only (`v1alpha2.rs:282`, `standing.rs:1756`).
- **I2:** 2,657 answers equal `regex::Regex` on `\A(?:p)\z`, 194 of them matches.
- **I3:** a stored `max_value_bytes` is refused by reader and command, also when equal to the computed one. `grant-standing` refuses exactly where the reader's bound is under 64, over three `\B` sweeps.
- **I4 CPU:** 25 families at their bounds took 26–302 ms; `^x*x{150}$` took 165–289 ms in the same runs.
- **I4 memory:** F2's 4,393,287 was the highest of 8 shapes. Nothing is per CPU by code reading: no pool, and the cache is made per check.
- **I5:** for 10 families the largest size the reader takes equals what the `regex` crate compiles at `size_limit` 65,536.
- **I7:** only `max_value_bytes` is added, and only to a pattern member (`approvals.rs:75–96`).

**The 13 edited assertions: still fails on its defect?**
I ran no mutant (no production edits); "fails" below means the fix agent's logs in `…/pattern/fix2/mutants/`.
1. yes: diff against the pre-fix-2 copy is the rename and two comment lines.
2. yes: `mutant-bytes.log` fails it, and it reads the five texts.
3, 4, 5. yes: `mutant-nobound.log` (job 4172) fails each. My `the_long_values_of_the_edited_cases…` shows each long value matches under `regex::Regex` and is refused as `TooLong` only. The 1 s assertion at the bound is kept.
6. yes: the nobound and foldoutlives mutants fail it; `matched >= 100` is kept.
7. yes: `mutant-cachekept.log` (job 4299) fails it; the band went from 262,144 to 65,536.
8. yes: nobound and cachekept fail it.
9. yes: cachekept fails it; the 1 % band is unchanged.
10. yes: cachekept fails both cases.
11. yes: cachekept and nosizelimit fail it.
12. yes: nobound, cachekept and nosizelimit fail it.
13. new case with no earlier defect: nosizelimit fails it, cachekept does not.

**Tests written** (all green; times from alone-7):
- `a_value_over_its_bound_is_outside_every_pattern` 0.02 s
- `a_served_write_over_the_bound_is_approval_required_and_reaches_no_provider` 0.59 s
- `the_whole_value_matches_under_every_anchor_a_signer_can_write` 0.01 s
- `a_stored_max_value_bytes_is_refused_not_read` 0.01 s
- `grant_standing_signs_exactly_the_patterns_the_reader_runs_on_64_bytes` 0.09 s
- `one_check_at_the_bound_takes_under_a_second_for_unicode_word_boundaries` 1.23 s
- `one_check_at_the_bound_takes_under_a_second_for_classes_and_unions` 1.34 s
- `one_check_at_the_bound_takes_under_a_second_for_anchors_and_nested_repetition` 1.16 s
- `one_read_of_a_file_of_age_classes_takes_under_a_second` 2.73 s (least of three reads; see F1 for its margin)
- `one_read_and_one_checked_write_peak_under_5_mib_for_other_shapes` 0.57 s
- `the_fold_rules_of_the_texts_are_the_readers` 0.01 s
- `the_reader_takes_what_the_regex_crate_compiles_under_the_size_limit` 0.06 s
- `the_long_values_of_the_edited_cases_are_outside_by_the_bound_alone` 0.09 s
- `a_listed_member_adds_max_value_bytes_to_a_pattern_and_nothing_else` 0.01 s

**Not checked**
- Whether my cases fail on a mutant.
- The pre-fix-2 reader was not built; I5 rests on the `regex` crate comparison and code reading.
- The CLI's `list-standing` output and the local owner's `TooLong` path end to end (code read only: `execution.rs:137`).
- `(?-u:\B)`, the read time under load, generated and website files, and the gate.
- At 09:21:54 CET `worktree pattern-grant/crates/connectors-host/src/local/standing.rs:266` held `text.chars().count()` where my tree has `text.len()`. I take it for the adversary's temporary mutant; the two trees were not equal when I finished.
- My lease is released.

```findings
- {file: contracts/service/compatibility.md, line: 56, category: resource-exhaustion, severity: warning, verdict: CONFIRMED, origin: introduced, message: "One read of 64 members of ^[^\\P{age=16.0}x20 a]$ takes 0.36 s CPU (0.61 s once), about 7 times the pattern the suite times; acceptance 25 holds with a margin of 1.6 to 2.8 times; no text states a read figure and the local owner reads up to three times per write."}
- {file: contracts/service/compatibility.md, line: 56, category: documentation, severity: note, verdict: CONFIRMED, origin: introduced, message: "Stated highest peak 4,251,225 bytes is exceeded: 4,315,312 (64 x any-run plus 64-range class {111}) and 4,393,287 (63 costliest plus one nested \\w member); the 5 MiB bound holds."}
- {file: docs/local-approvals.md, line: 178, category: documentation, severity: note, verdict: CONFIRMED, origin: introduced, message: "Byte-mode refusals also cover \\D, \\S, [[:^alpha:]] and [^\\W]; an ASCII difference class is taken and ASCII only. No pattern the texts call refused is taken."}
- {file: crates/connectors-host/src/local/standing.rs, line: 539, category: usability, severity: note, verdict: CONFIRMED, origin: introduced, message: "^\\w{2}$ and ^\\pL{2}$ exceed the compiled-size limit and are refused; the texts do not say so. Fail closed, not a security defect."}
```
