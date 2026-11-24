---
format: aep.planning-md/3
id: review-result:pattern-grant-adversary-round-3
kind: review-result
status: active
title: adversary review, round 3 (after fix round 2), story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
unit: fix round 2 of story:standing-grant-binds-a-pattern-or-a-value-set, working tree `worktree pattern-grant` (base d45310985 plus the uncommitted change)
verdict: CONFIRMED (no blocker; 1 warning, 7 notes)
cases: executed 157→169, red 0
origin: introduced 8 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 directory (scratch, listed below)
needs-coordinator: F1 needs a test outside my one file; F8 needs the generators (D-e)

**Diff stat.** `git --no-pager diff --stat`: 14 files, 804 insertions, 78 deletions, none mine. My only tree change is the untracked `crates/connectors-host/tests/standing_pattern_adversary_review_r3.rs` (909 lines). Nothing was staged or committed. `standing.rs` was edited 9 times for the mutants the brief ordered (the role file forbids mutating, the brief orders it); each edit was reversed and `cmp` with the `pattern-mutants` copy is empty (09:31 CET). My lease is released.

**Runs** (build-1d; job ids as `slot status` showed them, last four digits):
- Lane, job 5095, 09:28:25–09:29:28, exit 0: `Summary [ 11.686s] 169 tests run: 169 passed, 2043 skipped`.
- Clippy, job 5115: exit 0.
- `cargo fmt --check`: exit 1 on my file only; I ran `rustfmt` on that file, then exit 0.
- My file alone after formatting, job 5128, 09:30:17–09:30:58: `12 tests run: 12 passed`. The lane ran before the formatting.

**Findings**
- **F1, warning** — `apps/connectors/src/local/approvals.rs:80`. No test pins that `list-standing` prints `max_value_bytes`: putting `"target": grant.target` back fails nothing.
  - Evidence (code read, mutant not run): `max_value_bytes` is in no test under `apps/` or `adapters/`. `adapters/catalog/tests/local_runtime/slack_standing.rs:626` asserts only the length and `grant_id`. `apps/connectors/tests/standing_grant.rs:115` covers only the refusal without a keyring.
  - Fix: one assertion on `held[0]["target"]` in the catalog case, with a pattern grant.
- **F2, note** — `crates/connectors-host/src/local/standing.rs:266`. The bound counted in characters survives the suite as it was before my file. Job 4978: `118 passed, 2 failed`, both in my file. Closed if my file is kept.
- **F3, note** — `standing.rs:605`. Dropping the guard inside the public `Pattern::matches` survives the suite as it was. Job 5070: `120 passed, 1 failed`, mine. Closed likewise.
- **F4, note** — `standing.rs:908`. `TooLong` ranked below `OtherTarget` survives the suite as it was: no earlier case has two grants. Job 5083: `120 passed, 1 failed`, mine. Closed likewise.
- **F5, note** — `contracts/service/compatibility.md:56`. Before my file no test tied these numbers to the reader:
  - the peak 4,251,225: `standing_pattern_adversary_review_r2.rs:1372` prints it and asserts only ≤ 5 MiB;
  - the value sizes 1,803 / 1,611 / 1,696;
  - the three bounds as the five texts write them.
  - My cases pin them now, all green; the measured peak is 4,250,446. 2,293,760, the weights, the three bounds as code values and the five memory figures were pinned already.
- **F6, note** — `compatibility.md:56` and `contracts/cli/v1alpha1/semantics.md:976`. "157 ms to 399 ms" is pinned by no test and cannot be; the tests assert under 1 s. My runs gave 169.7 ms for `^x*x{150}$`, inside the range.
- **F7, note** — `standing_pattern_adversary_review_r2.rs:426`. The fix report's claim for edit 2 is wider than the case: it fails on one exact old sentence or four missing phrases, not on any text that calls a refused pattern taken. I read the five texts: every refused pattern they name is called refused.
- **F8, note** — `apps/connectors-cli-contract/binding.json` has no `max_value_bytes` (stale generated file, declared under D-e).

**Mutants** (filter `test(~local::standing::) | binary(~standing_pattern)`; "earlier" means the 109 cases that existed before my file):

| Mutant | Job | Result | Killed by |
|---|---|---|---|
| `>=` in `refuses` | 4940 | 109 passed, 11 failed | 5 earlier, 6 mine |
| `>=` in `matches` | 4957 | 108 passed, 12 failed | 5 earlier, 7 mine |
| bound in characters | 4978 | 118 passed, 2 failed | mine only |
| generic reason | 4997 | 114 passed, 6 failed | 2 earlier, 4 mine |
| floor 63 | 5024 | 117 passed, 3 failed | 1 earlier, 2 mine |
| floor 65 | 5040 | 119 passed, 2 failed | 1 earlier, 1 mine |
| reader bound doubled | 5056 | 114 passed, 7 failed | 1 earlier, 6 mine |
| inner guard dropped | 5070 | 120 passed, 1 failed | mine only |
| rank 4 → 2 | 5083 | 120 passed, 1 failed | mine only |

**The 13 edited assertions: each still fails on its defect — yes, all 13.**
- 1: yes. The diff shows the name and one doc line only.
- 2: yes. Four patterns refused, their group spellings taken, five texts read (see F7).
- 3, 4, 5: yes. Without a bound the value covers 64 where each now asserts 0; the 1 s bound is kept; both `>=` mutants fail all three (jobs 4940, 4957). Edit 4's floor is 8,192, not the 16,384 of D-a; the fix report cites the coordinator's order, which I cannot verify.
- 6: yes. `answer == (len <= bound && is_match)`; any other difference from `regex::Regex` fails.
- 7: yes, tighter: band 262,144 → 65,536.
- 8, 12: yes. `holds <= stated` is kept, on more values.
- 9: yes. Same 1 % band, new figures.
- 10: yes, tighter: band min(2 %, 4,096) against the stated figure itself.
- 11: yes. Two bounds added.
- 13: new case; it asserts only ≤ 5 MiB (F5).
- For 2, 3 and 10–13 I diffed against the 07:25 copy. For 4–6 no copy from before fix round 2 exists, so they are judged from the current text and the mutant runs.

**Tests I wrote** (12, all green now, none over 5 s; times from job 5095):
- `a_word_boundary_between_characters_outside_the_word_class_is_checked_within_a_second` 1.2 s
- `a_word_boundary_costs_no_more_per_unit_of_weight_than_a_run_of_live_byte_ranges` 1.9 s
- `the_reader_answers_as_the_regex_crate_does_for_every_value_within_the_bound` 0.09 s
- `grant_standing_signs_a_pattern_exactly_when_the_reader_matches_it_against_64_bytes` 0.07 s
- `one_read_and_one_checked_write_peak_at_the_figure_the_contract_states` 0.07 s
- `a_member_the_command_would_not_sign_covers_values_up_to_its_own_bound` 0.03 s
- Under 0.02 s each: `the_bound_counts_the_bytes_of_a_value_and_not_its_characters`, `a_pattern_asked_directly_matches_no_value_longer_than_its_bound`, `two_pattern_members_of_one_grant_are_each_held_to_their_own_bound`, `a_value_too_long_for_one_grant_is_named_so_beside_a_grant_it_is_outside`, `a_listed_member_holds_max_value_bytes_exactly_when_it_is_a_pattern`, `the_texts_state_the_bounds_the_reader_computes`

**Attacked and not broken**
- Point 1: 64 members at their bound, test profile, 09:13–09:14 CET. `^-*(?:\B{8}-){10}$` on 845 bytes took 138.6 ms; `^-*\B{120}-$` on 585 bytes 134.1 ms; `^×*(?:\B{8}×){10}$` on 818 bytes 131.2 ms; the reference `^x*x{150}$` on 1,803 bytes 169.7 ms. Cost per unit of weight is 0.79 of the reference at most. Nothing over 1 s.
- Point 2: the exact bound and one byte over; 2-, 3- and 4-byte characters; the empty value; each member held to its own bound; a 30,000-byte `one_of` string compared whole.
- Point 3: 29 patterns, 13,282 values, equal to `regex::Regex` on `\A(?:pattern)\z` within the bound.
- Point 4: `grant-standing` refuses exactly when the reader's bound is under 64 (real patterns of bound 64 and 63). A hand-written under-64 member covers matching values up to its bound and is `TooLong` above, as `semantics.md:974` says. The texts' own example needs 200 bytes against a bound of 43, so it covers nothing.
- Point 5: the code, `semantics.md:1026` and `ess/domains/cli.yaml:128` agree; a value-set or exact member has no `max_value_bytes`.

**Not checked**
- The CLI's `list-standing` output end to end (F1).
- Generators, drift checks, the gate, the whole workspace.
- Mutants outside `standing.rs`.
- CPU time on a loaded node beyond my two runs, and pattern families beyond those above and the fix agent's.

**Written outside the worktree**: `<session scratch>/pattern/r3-adversary/` — the logs `own1.log`, `own2.log`, `lane.log`, `clippy.log`, `fmt.log`, `m-*.log`; the job lines `*.job`; and `mutant.sh`, `summary.sh`, `jobid.sh`, `narrow.args`.

```findings
- file: apps/connectors/src/local/approvals.rs
  line: 80
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: no test pins that list-standing prints max_value_bytes; reverting the target to the raw member fails nothing
- file: crates/connectors-host/src/local/standing.rs
  line: 266
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: a bound counted in characters survives the suite as it was before the r3 file, which now kills it
- file: crates/connectors-host/src/local/standing.rs
  line: 605
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: dropping the length guard of the public Pattern::matches survives the suite as it was before the r3 file, which now kills it
- file: crates/connectors-host/src/local/standing.rs
  line: 908
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: TooLong ranked below OtherTarget survives the suite as it was before the r3 file, which now kills it
- file: contracts/service/compatibility.md
  line: 56
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the peak 4,251,225, the value sizes 1,803 / 1,611 / 1,696 and the bounds as the texts write them were tied to the reader by no test before the r3 file
- file: contracts/cli/v1alpha1/semantics.md
  line: 976
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the CPU range 157 ms to 399 ms is pinned by no test and cannot be; the tests assert under 1 s
- file: crates/connectors-host/tests/standing_pattern_adversary_review_r2.rs
  line: 426
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the edited fold case fails on one exact old sentence or four missing phrases, not on any text that calls a refused pattern taken, as the fix report claims
- file: apps/connectors-cli-contract/binding.json
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the generated binding lacks max_value_bytes of StandingTargetView until the generators run
```
