---
format: aep.planning-md/3
id: review-result:pattern-grant-adversary-round-1
kind: review-result
status: active
title: Adversary review, round 1, story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
verdict: needs-change

No invariant broke on the consumer's two grants. Four cases are red: two show the suite cannot tell the whole-value wrap from its absence, two show contract sentences that are false for patterns a signer may write.

- unit: story:standing-grant-binds-a-pattern-or-a-value-set, working tree against d45310985 (`git diff --cached --stat`: 57 files changed, 2559 insertions(+), 144 deletions(-))
- cases: 113 before, 129 after, red 4 (mine); origin: introduced 4, pre-existing 0
- needs-coordinator: no. Lease `review-adversary-4fb1ea8b` released.

```findings
- file: crates/connectors-host/src/local/standing.rs
  line: 358
  category: test-gap
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'No test tells the wrap `\A(?:pattern)\z` from matching the pattern as written. Mutant M1 (both forms compile, `whole = alone`) in a copy of the tree: the story''s tests give `Summary [ 26.745s] 101 tests run: 101 passed, 776 skipped`. Under M1 `^DEV-[0-9]+|x$` admits `DEV-12/../admin`. Test a_shared_vector_or_a_suite_case_needs_the_wrap_to_hold (crates/connectors-host/tests/standing_pattern_adversary_review.rs:238): `none of 73 cases tells \A(?:pattern)\z from the pattern as written`. Reached by any later edit of the host''s copy of the rule; the catalog has such cases (amendment.rs:1086-1099), the host and the 42 shared vectors do not. Fix: add `^a|b$` on `ab` (false) and `^(?m)a$` on `a\nb` (false) to fixtures/whole_value_patterns.json.'
- file: crates/connectors-host/src/local/standing.rs
  line: 357
  category: test-gap
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: 'No test tells whether the pattern is compiled on its own before the wrap. Mutant M2 (line 357 deleted): `Summary [ 50.764s] 101 tests run: 101 passed (1 slow), 776 skipped`. Under M2 `^a)|(?:b$` is taken and, as `\A(?:^a)|(?:b$)\z`, admits `a/../x` and `xb`. Test a_shared_vector_or_a_suite_case_needs_the_compile_on_its_own (same file:254): `none of 73 cases tells whether the pattern is compiled on its own`. Fix: add the vector `^a)|(?:b$` (matches null).'
- file: docs/local-approvals.md
  line: 180
  category: contract
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: 'Invariant 1, a look-alike through a case fold. `^(?i)sk-[0-9]+$` admits `\u{17f}k-12`, `s\u{212a}-12` and `\u{17f}\u{212a}-12` (long s, Kelvin sign): no ASCII s or k in the value. The texts name only `\d \w \s \b` as Unicode-aware (also contracts/cli/v1alpha1/semantics.md:968, docs/running-services.md:324, ess/domains/standing_approvals.yaml:46) and the shared vector `^(?i)dev$` shows `(?i)` as the way to fold. Test a_case_insensitive_pattern_admits_no_unicode_look_alike (same file:310), red: `^(?i)sk-[0-9]+$ admits values that hold no ASCII s or k`. Reached only by a signer who writes `(?i)`; the consumer''s two grants do not. Fix: one sentence in the four texts that `(?i)` folds Unicode and `(?i-u:...)` folds ASCII only (green: an_ascii_case_fold_is_written_with_the_unicode_flag_off), or refuse `(?i` without `-u`.'
- file: docs/local-approvals.md
  line: 181
  category: contract
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: '"`.` does not match a line break" is false: `^Release review of .*$` admits a value holding U+000D, U+000B, U+000C, U+0085, U+2028 or U+2029; only U+000A is not crossed. Test a_dot_crosses_no_line_break (same file:353), red: `. matched these line breaks left: [\u{d}, \u{b}, \u{c}, \u{85}, \u{2028}, \u{2029}]`. Reached by a signer who writes `.*` to hold one line. Fix: write "`.` matches every character but U+000A".'
```

**Tests written** (`worktree pattern-grant/crates/connectors-host/tests/standing_pattern_adversary_review.rs`, 16 tests, no unit-test module added):
- Red, 4: the four named above.
- Green, 12: the rest. `one_anchor_per_branch_and_a_multi_line_flag_still_hold_the_whole_value` and `a_flag_or_a_comment_inside_a_pattern_opens_nothing` turn red under both mutants. Under M1 the review binary gives `16 tests run: 10 passed, 6 failed`, under M2 `8 passed, 8 failed`.
- Over 5 s: `a_served_pattern_grant_holds_one_cap_and_ends_as_an_exact_one`, 14.5 s alone and 7.7 s in the lane.

**Commands** (through `slot --timeout 30m sh <script>`):
- Own binary alone, load 73.49: `Summary [ 14.552s] 16 tests run: 12 passed, 4 failed, 0 skipped`.
- Lane of the brief, load 63.86: `Summary [ 85.393s] 129 tests run: 122 passed (1 slow), 7 failed, 2043 skipped`.
  - 4 failures are mine and 2 are the security reviewer's (`one_read_of_a_file_within_the_stated_limits_...`).
  - 1 is the known `replays_waiting_on_an_original_hold_no_slot_and_at_most_four_wait_per_key`, 45.6 s, "the replays did not join their original".
- Mutants M1 and M2, load 43.08 and 31.71: result lines in the findings.

**Tried and held:**
1. `^DEV-[0-9]+$` refuses 18 look-alike or other-form values (fullwidth and Arabic digits, zero-width space, BOM, NUL, CR, LF, Cyrillic E, U+2010, combining mark, spaces).
2. Raw-byte inputs: `"DEV-12\u000a"`, a number, a list, an object, null, `/K` and `/k ` are refused; `"\u0044EV-12"` is admitted; a duplicate key is no input.
3. Pointer spellings: `/a~1b`, `/m~0n`, `/list/0` against `/list/00`, `/list/+0`, `/list/-`, `/list/0x0`, `/list/1e0`.
4. 23 flag and comment patterns (`(?x)` comment, `(?m)`, `^a|b$`, `^|$`, `^(?m)$`, `^a\z$`) are each refused or hold the whole value.
5. `one_of:[b]` equals `value:b` over 22 bindable by 31 sent values; `1e4`, `10000.0`, `-0`, `0.0`, `1.5`, 2^64, -2^63-1 and a nested `2.0` are refused in both forms.
6. 39 member texts (duplicate key, unknown key, wrong type, two forms, none, array form with 1, 3 or 4 elements, pattern with a space or newline, `{907}`, bad sets) refuse the whole file, in first, last and other-instance position.
7. The 65th pattern member refuses the file in an expired grant, a revoked grant, another instance and another operation; `one_of` and `value` members beside 64 patterns do not count.
8. `check_request` (local `grant-standing`) and `parse` (served reader) take the same 7 and refuse the same 39 members.
9. Array-index relaxation, 4,000 fixed-sequence inputs: the grant with the member covers exactly the inputs the grant without it covers whose `/a/list/0/text` is `x`.
10. Served host: 13 writes outside the pattern or set get 403 with 0 provider requests and 0 uses.
11. Served cap of 3: DEV-1..3 admitted under one counter; DEV-4 and DEV-1 with a new key get 429; a replay at the cap is replayed with no use; a higher cap admits again and earlier uses still count.
12. Served end of grant: revocation and `not_after` give 403.

**Not looked at:**
- Invariant 6 (time and memory): left to the security reviewer's probe.
- Local `admit` through a signed file and the `connectors` CLI binary end to end: read only, not driven.
- Invariant 5: read only (`execution.rs:184-229`).
- A 0.77.0 reader refusing the new members: read at d45310985 (`deny_unknown_fields`, overlap refused), not run, so unproven by test.
- Generated files.
- Unproven notes, no test: `^a\\$` is refused although its `$` is unescaped (standing.rs:346, fail-closed); a member written as `["/k","x"]` reads as an exact member (pre-existing).

**Written outside the worktree**, all under `<session scratch>/pattern/review-adversary/`:
- `standing_base.rs`, `standing.orig.rs`, `standing.M1.rs`, `standing.M2.rs`
- `run1.sh`, `run2.sh`, `mutants.sh`
- `own.log`, `lane.log`, `mutant.M1.log`, `mutant.M2.log`, `run1.slot.log`, `run2.slot.log`
- `mutant/` (71 MB copy of the tree) and `mutant-target/` (978 MB)
