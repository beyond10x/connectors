---
format: aep.planning-md/3
id: review-result:standing-grant-local-security-1
kind: review-result
status: active
title: Security review, round 1, story:local-invoke-admits-a-keyed-write-under-a-standing-grant
relations:
- reviews: story:local-invoke-admits-a-keyed-write-under-a-standing-grant
revision: 1
---
verdict: needs-change

```findings
- file: crates/connectors-host/src/local/standing.rs
  line: 540
  category: integrity
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "Removing <state>/standing-grants.sqlite3 frees the hourly cap: Uses::open (:540, create_private :737-746) recreates the store empty and the owner opens it per write (owner/mutation/execution.rs:211), so a grant of 1 write an hour admits a second write in the same hour, and replays lose mutation.standing. Shown by security_sg_a_removed_use_store_does_not_free_the_hourly_cap (fails at :3396, 'Admitted(\"g1\")'). Reached by any process of the owner's UID, which the invoking caller is; no key needed. The Design's Boundary accepts the same UID, but the contract states only the grant-file rollback limit (contracts/cli/v1alpha1/semantics.md:1100-1105), not this one. Fix: a contract line beside the rollback limit and the test inverted, or the owner refuses an absent store as unavailable and grant-standing creates it."
- file: crates/connectors-host/src/local/standing.rs
  line: 336
  category: boundary
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "A target value is not matched digit for digit: read_json turns an integer outside i64/u64 and every fraction into the nearest f64 (crates/connectors-core/src/lib.rs:257-264), the grant is compared on that value (:336), and the adapter is sent the caller's bytes (execution.rs:416). security_sg_a_bound_number_is_matched_digit_for_digit fails at :3506: a grant for 18446744073709551616 admits 18446744073709551617. The state was built; nothing found reaches a numeric target of that size (catalog targets are strings). Fix: parse/requested refuse a target value holding a number that is not an exact i64/u64, or the contract says numbers compare as f64."
- file: contracts/cli/v1alpha1/semantics.md
  line: 977
  category: contract-drift
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "grant-standing and revoke-standing can answer two codes their lists (:977-989, :998-1002) do not name: outcome_unknown when the replace fails after the rename (standing.rs:964, approval_issuance.rs:609) and forbidden when the keyring pair is not the configured key (standing.rs:870, approval_issuance.rs:608). Read only for the CLI; the forbidden half is shown at the library by security_sg_only_the_configured_pair_signs_a_grant_file. Fix: two lines in the contract and docs/local-approvals.md."
```

Unit: commits 9fbce06c9 and e1edc836e on `story/standing-grant-local`, 2026-11-23. The 7 tests
named `security_sg_*` are in module `signed_tests` of
`crates/connectors-host/src/local/standing.rs` (commit a663e9749): 5 pass, 2 fail and name the
first two findings.

Points:

1. Nothing self-issues: held by test and reading. Another pair or another key id is `forbidden`
   and nothing is written. A signer is built only from the keyring seed, reached only from the
   CLI's approvals commands. From the public key or from a proof no grant file can be made. An
   older signed file can be put back (point 2).
2. Signature and canonical form: held by test (22 respellings of one signed file admit
   nothing). A rollback to an older signed file is possible and is bounded by
   `not_after_unix_ms` and by key rotation; uses taken before the revocation still count.
3. Fail closed: held by test for custody before bytes, a file over 64 KiB and five broken store
   forms; by reading for the rest.
4. A present proof never falls back to a grant: held by reading.
5. Scope: held by reading. A reconnect keeps grants and a key rotation ends them.
6. Cap: held by test for a restart, a second handle, a clock set back and a rewrite. Broken
   for removal of the store (finding 1). Editing its rows is the same class and was not run.
7. Nothing secret leaves in the log or the answers: held by reading.
8. Audit: the record is a bound row `grant_uses(attempt_id, grant_id)` in the use store,
   written before dispatch, and `mutation.standing.grant_id` in the answer; the ledger says
   `not_required`.
9. Members a consumer reads: `result.mutation` gains an optional `standing`;
   `error.data.code` gains the value `rate_limited`; `error.data.retry_after_seconds` is new
   and present only with `rate_limited`; `result.result.status` and
   `error.data.mutation.classification` are unchanged.

Not established: nothing was run against the base; no CLI-level run; no mutants; the macOS and
Windows read paths were not run; the owner's refusal mapping in `execution.rs` was read, not
driven.
