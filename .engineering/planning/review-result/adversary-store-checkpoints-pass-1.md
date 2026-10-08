---
format: aep.planning-md/3
id: review-result:adversary-store-checkpoints-pass-1
kind: review-result
status: active
title: Adversary pass 1 on store checkpoints and owner-routed reads
relations:
- reviews: story:metadata-invoke-cost-flat-in-store-size
revision: 1
---
## Adversary pass 1 on story:metadata-invoke-cost-flat-in-store-size

Tree `conn-u2-ckpt` at `a6a59980e4` plus 2 uncommitted test files, 2026-10-08. Report as returned
by `aep:adversary`, findings block verbatim.

unit: story:metadata-invoke-cost-flat-in-store-size
verdict: CONFIRMED (1 red case, severity warning; no blocker found)
cases: executed 515→519, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: yes. Non-owner processes open from a checkpoint at the provider level, but the story's Integrity text says they keep a FullVerification open (finding J1).

Cases added (uncommitted): `crates/connectors-host/tests/pooled_open_scan_adversary.rs`
(`a_damaged_page_written_between_two_opens_in_one_process_is_refused`, red) and
`apps/connectors/tests/checkpoints_owner_route_adversary.rs`
(`routed_metadata_reads_answer_as_the_direct_read_does`, `a_real_running_owner_refuses_the_enable`,
`a_confirmation_other_than_one_way_is_cli_input`, green).

Suite runs: `cargo test -p connectors-host --no-fail-fast` 393 passed, 1 failed, EXIT=101;
`cargo test -p connectors` 122 passed, 0 failed, EXIT=0.

- R1 `crates/connectors-host/src/local/metadata.rs:1030` (`recorded || !self.pooled`), with
  `er.rs:589` `holds()`: inside a process that holds a pooled handle, page-level damage to 22
  b-trees (including Eventlog's `capture_continuity` and `capture_journal`) is accepted. What
  reaches it: only the owner keeps a pooled handle for long; the trigger is a disk fault or a
  non-SQLite writer while an owner runs; no product path writes like this. The next fresh open
  still refuses it. CONFIRMED, introduced, warning.
- J1 `checkpoint_tests.rs:99` asserts a non-owner `Metadata::inspect` has
  `opened_from_checkpoint()`; the story's Integrity section says every other open keeps a
  FullVerification open. The code does a ProviderTracked checkpoint open then
  `complete_snapshot`; Entity Runtime 0.29.0 `tracked.rs:376` makes that a complete read, so the
  adversary believes they are equivalent, but the stated rule is not what was built.
  NEEDS-CHANGE, introduced, note.
- J2 `er.rs:3438-3441`: the expiry-only path with more than K expiries has no test; dropping
  `actions.append(&mut tail)` leaves the suite green. CONFIRMED, introduced, warning.
- J3 `owner.rs:257-262`: a `cached` request failing after a successful greeting returns the error
  instead of falling back to the direct read `owner.md` promises. INFEASIBLE, introduced, note.
- J4 `docs/local-er-metadata.md:254`: a 4-cell row in a 3-column table. CONFIRMED, introduced,
  note.

Not broken: file replaced at the same path; raw SQLite edit while a handle is held; offline edit
then owner start or direct command; Cached from an owner of another config or store; owner of a
different build; admission before owner start; IN_OWNER; expiry chunk accounting;
checkpoints-enable wrong confirmation, running owner, second run.

```findings
- file: crates/connectors-host/src/local/metadata.rs
  line: 1030
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a pooled open skips quick_check, so page-level damage to 22 b-trees including the capture continuity and journal tables is accepted inside a process holding a verified handle, where every open used to refuse it
- file: crates/connectors-host/src/local/metadata/checkpoint_tests.rs
  line: 99
  category: acceptance
  severity: note
  verdict: NEEDS-CHANGE
  origin: introduced
  message: non-owner opens start from a provider checkpoint plus a host complete snapshot, while the story's Integrity section requires them to keep a FullVerification open
- file: crates/connectors-host/src/local/metadata/er.rs
  line: 3438
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the expiry-only path (no caller actions, more than K expiries, last chunk as final batch) has no test, and dropping the tail append would leave the suite green
- file: crates/connectors-host/src/local/owner.rs
  line: 257
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: a cached request that fails after a successful greeting returns the owner error instead of falling back to the direct read the owner contract promises
- file: docs/local-er-metadata.md
  line: 254
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the pooled-handle row has four cells in a three-column table, so the quick_check statement renders outside the table
```
