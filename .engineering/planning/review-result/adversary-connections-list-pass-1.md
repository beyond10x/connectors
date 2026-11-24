---
format: aep.planning-md/3
id: review-result:adversary-connections-list-pass-1
kind: review-result
status: active
title: Adversary pass 1 on connections list through the owner
relations:
- reviews: story:connections-list-answers-through-the-owner
revision: 1
---
unit: story:connections-list-answers-through-the-owner, uncommitted tree connections-through-owner on c78ad178ff1
verdict: approve

```findings
- file: crates/connectors-host/src/local/owner/transport.rs
  line: 3143
  category: test-gap
  severity: warning
  verdict: approve
  origin: introduced
  message: "No test of the unit ran the owner's real dispatch of `observe` with a custody observation different from the owner's own. A mutant that judges custody in the owner (`custody::available_for(&config.selection())` in place of the request's `custody`) passed all three `owner_observe*` host tests; the jail tests were not run on it. `adversary_a_real_owner_exchange_observes_with_the_callers_custody_not_its_own` fails on that mutant and passes on the real tree. Fix: keep the test."
- file: apps/connectors/tests/cold_owner_start.rs
  line: 895
  category: test-gap
  severity: note
  verdict: approve
  origin: introduced
  message: "Not covered end to end before: paging across the two paths, a list right after `connections revoke` wrote the store beside the owner, a revoked connection, and custody lost in the command's view. `adversary_pages_a_revoked_connection_and_lost_custody_answer_alike_beside_the_owner_and_without` covers them and passes. Fix: keep the test. Acceptance 3's two-held case still has no CLI test beside an owner; the CLI cannot build that state without a race."
- file: crates/connectors-host/src/local/owner/transport.rs
  category: correctness
  severity: note
  verdict: approve
  origin: introduced
  message: "When the client's 30 s span ends, the owner's work is not cancelled. A `list` may still hold `metadata.lock` during a fresh complete open, and the fallback read then waits up to 30 s behind it: about twice the open time, or `metadata_unavailable` if the owner's open exceeds about 60 s. Read from the code; no test fits the 60 s bound (it needs an open over 30 s). Fix: a story of its own, or say so in owner.md."
- file: contracts/cli/v1alpha1/owner.md
  category: contract
  severity: note
  verdict: approve
  origin: introduced
  message: "The owner answers `observe` to a caller of any build; the same-build check is client-side only. Every socket peer is the same uid and can read the store itself, so nothing new is exposed. No test written. Fix: none needed."
```

Tests added, both green on the real tree, one run (26.5 s, exit 0):

- `PASS [ 5.073s] connectors-host local::owner::transport::admission_tests::adversary_a_real_owner_exchange_observes_with_the_callers_custody_not_its_own`. On the mutant copy: `FAIL [ 9.958s]`, the owner answered `Ready` where the caller's observation gives `CustodyUnavailable`.
- `PASS [ 26.458s] connectors::cold_owner_start adversary_pages_a_revoked_connection_and_lost_custody_answer_alike_beside_the_owner_and_without`.

Held:

- **By test:** through the owner there was no open and no snapshot, also right after a foreign revoke. Head, tail and both describes equal the direct answers. An owner-made cursor is redeemed directly. The `stale_cursor` refusal is the same on both paths. An unnamed describe with only a revoked connection gives `not_found/admission/create_connection`. No owner is started or replaced.
- **By reading:**
  - Every owner refusal or failure falls back to the direct read, so code, stage and next action are the direct read's.
  - Refusal order for an unnamed describe is unchanged.
  - Another instance's connection or acquisition is refused.
  - A missing store, a stale socket and an owner of another build all take the direct path.
  - The idle restart matches what `cached` already does.
  - `owner.md` matches the code.

`.engineering/planning/story/connections-list-answers-through-the-owner.md` changed at 13:14, after my run and not by me. The implementation files are unchanged since 11:02.

Written outside the worktree, all under `<local path>`: `run1.sh`, `run1.log`, `copy.sh`, `run-mutant.sh`, `run-mutant.log`, and `mutant/` (tree copy with the one mutated line; its `target/` is deleted). Lease released.

Edited in the checkout, tests only:
- `crates/connectors-host/src/local/owner/admission_tests.rs` (line 1000)
- `apps/connectors/tests/cold_owner_start.rs` (line 895)
