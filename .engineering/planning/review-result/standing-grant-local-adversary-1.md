---
format: aep.planning-md/3
id: review-result:standing-grant-local-adversary-1
kind: review-result
status: active
title: Adversary, round 1, story:local-invoke-admits-a-keyed-write-under-a-standing-grant
relations:
- reviews: story:local-invoke-admits-a-keyed-write-under-a-standing-grant
revision: 1
---
verdict: needs-change

```findings
- file: crates/connectors-host/src/local/owner/mutation/execution.rs
  line: 151
  category: test-gap
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "Making the owner's still_admitted return Ok(()) at once (mutant c, scratch copy) fails no test in `(test(standing) | test(mutation::)) - test(final_audit_recovery)`: 50 run, only a scratch probe red. Both rechecks in execute (lines 517 and 615) are held by reading alone. Fix: one test that drives execute with the grant file revoked between admit and prepare, and one between bind and the gate."
- file: crates/connectors-host/src/local/standing.rs
  line: 61
  category: rate-limit
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "Removing <state>/standing-grants.sqlite3 frees max_per_hour: a scratch probe got Admitted(g1) where RateLimited was expected, and security_sg_a_removed_use_store_does_not_free_the_hourly_cap (the security reviewer's) is red on the real tree. Only the operator's UID can remove it, which the contract's same-user boundary allows, but semantics.md line 1094 does not name it. Fix: name it beside 'What revocation does not undo', or refuse a grant write when the store is absent and the ledger holds an attempt under a grant."
- file: crates/connectors-host/src/local/standing.rs
  line: 1255
  category: test-gap
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Counting the use at now.lower_unix_ms instead of the upper bound (mutant e) left the committed suite green; only adversary_sg_the_hour_is_counted_from_the_upper_bound_of_each_reading fails. Fix: keep that test."
- file: crates/connectors-host/src/local/standing.rs
  line: 998
  category: test-gap
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Changing not_after_unix_ms < 1 to < 0 in requested (mutant a) left the committed suite green; only adversary_sg_check_request_refuses_without_a_clock fails. Fix: keep that test."
- file: crates/connectors-host/src/local/standing.rs
  line: 3380
  category: input-binding
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "security_sg_a_bound_number_is_matched_digit_for_digit (the security reviewer's test) is red on the real tree. read_json turns an integer outside i64/u64 into the nearest f64, so two digit strings compare equal; the adapter reads the same bytes with the same reader, so no gap between the checked and the dispatched value was shown. Fix: refuse a bound number that is not exactly representable, or compare the digits."
```

Unit: commits 9fbce06c9 and e1edc836e on `story/standing-grant-local`, 2026-11-23. The 11 tests
named `adversary_sg_*` are in module `signed_tests` of
`crates/connectors-host/src/local/standing.rs` (commit a663e9749); on the real tree all 11 pass
in 1.7 s.

Attack points:

1. Counting: held by test (one owner, four processes, a new handle, a rewrite of the file, the
   hour boundary, a clock stepped back, release once, a panic before bind). Broken for a removed
   use store (finding 2). Replay of a key: held by reading.
2. No fallback from a present proof to a grant: held by test at `standing::route` and by the
   committed `a_present_proof_never_falls_back_to_a_grant`; in `execute` held by reading.
3. Input binding: held by test (26 cases and the duplicate member). Broken for numbers past 64
   bits (finding 5).
4. The file between check and use: `standing::still_admitted` held by test (14 files); the
   owner's two calls held by reading only (finding 1).
5. Scope: instance, operation and another key under the same key id held by test; another
   connector with the same operation name and a reconnect held by reading.
6. Unchanged behaviour without the member: held by reading.
7. Mutants (scratch copy): b (no release on drop), d (signature not checked) and g (another
   grant stands in at the recheck) each fail a committed test; a and e fail only the
   adversary's tests; c fails no test.

Not established: `binary(standing_grant)` and the journeys were not run; no test of the pass
calls `execute`.

A defect of the pass itself, repaired: from 22:44:47 to about 23:31 CET the pass built its
scratch copy into the checkout's `target` directory, so every `-p connectors-host --lib` run in
the checkout in that window used a binary built from the copy (with a mutant from 22:47). Runs
of that window do not count. The artifacts were rebuilt from the checkout afterwards.
