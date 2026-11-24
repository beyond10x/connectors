---
format: aep.planning-md/3
id: review-result:pattern-grant-security-round-1
kind: review-result
status: active
title: Security review, round 1, story:standing-grant-binds-a-pattern-or-a-value-set
relations:
- reviews: story:standing-grant-binds-a-pattern-or-a-value-set
revision: 1
---
verdict: needs-change

Tree: `worktree pattern-grant`, working tree against d45310985. I wrote one file there, untracked: `crates/connectors-host/tests/standing_pattern_security_review.rs`. `git diff --stat` is empty; no implementation file was touched and no `review_security_tests` module was needed (the reader is public). cases: executed 108 (implementor's figure) → 113, red 2.

```findings
- file: crates/connectors-host/src/local/standing.rs
  line: 351
  category: contract
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "One read of a file inside every stated limit holds 417,415,230 bytes, 15.3 times the 27,278,992 the contract gives for the costliest pattern found (contracts/service/compatibility.md:56, contracts/cli/v1alpha1/semantics.md:971). Input: 64 grants, each one member with 100 empty groups then [^\\x00-\\x7f]?-{1725} (222 bytes, file 24,503 bytes), and one write whose value is 1,725 hyphens. Test one_read_of_a_file_within_the_stated_limits_holds_no_more_than_the_contract_states (standing_pattern_security_review.rs:156): FAIL [0.194s]. Cause: the fallback matcher's cache is states times capture slots, which size_limit and dfa_size_limit do not bound. Smallest fix: build through regex_automata::meta with onepass(false), backtrack(false) and WhichCaptures::Implicit; in a scratch probe the same pattern then holds 282,342 bytes and the worst of all candidates 323,474. That is a measured worst case, not a proven ceiling."
- file: crates/connectors-host/src/local/standing.rs
  line: 65
  category: contract
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "One read of a file inside every stated limit takes 1.56 to 1.85 s of thread CPU time in the test profile (opt-level 1), against the under-1-s bound of acceptance 25; the local path reads up to three times per write. Input: 64 members of ^(?i)[ then 82 times \\pL then ]$ (254 bytes, file 31,671 bytes). Test one_read_of_a_file_within_the_stated_limits_takes_under_a_second (:179): FAIL [3.665s]. The cost is case folding in regex-syntax, which no compile limit bounds; the implementor's test at standing.rs:2567 uses only ^[a-z]{906}$. No one-line fix was found: a lower PATTERN_LIMIT scales it down (not measured), or the contract states the measured time. Release CPU time not measured."
- file: crates/connectors-host/src/local/standing.rs
  line: 357
  category: test-gap
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "With compile(&text)? removed in a scratch copy, a file holding the pattern ^a)|(b$ is read (it wraps to \\A(?:^a)|(b$)\\z, a partial match), and every pattern test of the implementor still passes, including both the_shared_pattern_vectors_hold. My a_member_the_reader_does_not_understand_refuses_the_whole_file (:212) fails on that copy (left Ok(Ok(1)), right Err(Malformed)) and passes on the real tree. Smallest fix: add the vector to tests/fixtures/whole_value_patterns.json so the catalog's copy is held too."
```

**Judgement on the memory figures (invariant 6).** Publishing a measured figure instead of a ceiling is acceptable for a file only the approval-key holder or the mounting operator can write: no trust boundary is crossed, and I found no plausible grant that costs this much. The published figure is not acceptable, because it is 15 times under what an in-limit file holds. On a served host, any caller allowed to write decides when that cost is paid. Either bound the build as in finding 1, which brings the worst case found under the 26 MiB already stated, or correct the figure to 398 MiB.

**Tests I wrote** (all in `standing_pattern_security_review.rs`, run alone first):
- `one_read_of_a_file_within_the_stated_limits_holds_no_more_than_the_contract_states`: red, `FAIL [0.194s]`, holds 417,415,230 bytes.
- `one_read_of_a_file_within_the_stated_limits_takes_under_a_second`: red, `FAIL [3.665s]`, CPU 1.786 s and 1.849 s for two reads.
- `the_costliest_pattern_of_the_contract_holds_what_the_contract_states`: green, `PASS [0.102s]`. It is the control: `^\b\w\b$` × 64 measures inside 26,000,000 to 27,541,136 bytes with the same counter.
- `a_member_the_reader_does_not_understand_refuses_the_whole_file`: green, `PASS [0.014s]`.
- `a_pattern_holds_the_whole_value_under_every_flag_a_signer_can_write`: green, `PASS [0.014s]`.

**Test commands:**
- Own cases: `5 tests run: 3 passed, 2 failed, 0 skipped` in 3.665 s, load 17.2.
- Lane as the brief names it: `113 tests run: 111 passed, 2 failed, 2043 skipped` in 26.729 s, load 17.2 to 15.6. The two failures are mine. The adversary's file was not there yet.
- Base reader in a scratch copy of d45310985: `2 tests run: 2 passed, 824 skipped` in 0.019 s, load 49.
- Mutant copy, host and catalog only: `107 tests run: 101 passed, 5 failed, 1 timed out`, load 21 to 70. Three failures are mine. The other three are replay and wait tests (`review_replays_waiting_on_a_pending_key…`, `replays_waiting_on_an_original_hold_no_slot…`, `adversary_a_concurrent_duplicate_under_a_cap_of_one…` timed out at 60 s); they passed in the real tree at load 17, and I did not rerun them.

**What held:**
1. The 0.77.0 reader refuses a file with a `pattern` member, a `one_of` member, `value` beside either, an exact member beside a pattern member, an `input_keys` pointer above a target through an index, and the consumer's transition grant — alone and after an exact grant (run, scratch copy of the base).
2. The new reader refuses 31 bad member forms behind a valid exact grant: unknown key, two forms, `null`, wrong types, sequence form, unanchored, `(?x)` comment, uncompilable, 257 bytes, over the size limit.
3. A key written twice in a member is refused.
4. 64 pattern members read; 65 do not, also when the last sits in an expired grant.
5. `(?m)`, `(?R)`, `(?U)` and a top-level `|` give no partial or across-newline match.
6. No case fold or look-alike matches: fullwidth letters and digits, a combining mark, a zero-width joiner, NUL, surrounding spaces.
7. Non-string values never match a pattern, including `^(?s:.*)$`.
8. Pointer spellings `/list/01`, `/list/+1`, `/list/1.0`, `/list/-` reach no member.
9. A proof never falls back to a grant: `route` at `standing.rs:1465` and `execution.rs:288-306` (read), and `a_present_proof_never_falls_back_to_a_grant` passes in the lane.
10. An unsigned local file never reaches the compiler: the signature is verified before `parse` (`standing.rs:1222-1234`, read).
11. Served and local call the same `covering` (`service.rs:466`, `standing.rs:1581`), and nothing else deserializes a `Grant` (read).
12. Refusing capture groups alone is not enough for a ceiling: the most held with no group is 785,534 bytes (`^\b\p{Greek}{29}\b$`, probe).

**Not looked at:** the generated files; invariant 4 beyond reading `Uses::take`; the `apps/connectors` CLI mapping; concurrent served requests each holding their own read (not measured); macOS and Windows read paths; `docs/` wording beyond the bounds; the adversary's file.

**Written outside the checkout**, all under `<session scratch>/pattern/review-security/`:
- Probe: `probe/`, `probe-target/` (74 MB), `probe.sh`, `probe.log`, `probe.round1.log`, `probe.round2.log`.
- Base copy: `base/`, `base-target/` (798 MB), `base_tests.rs`, `base.sh`, `base.log`.
- Mutant copy: `mutant/`, `mutant-target/` (1.7 GB), `mutant.sh`, `mutant.log`.
- Runs: `run.sh`, `own.log`, `lane.log`, `standing.diff`, and the `*.slot.log` and `*.done` files.

My worktree lease is released.
