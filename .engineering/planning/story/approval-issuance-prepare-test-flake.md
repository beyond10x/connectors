---
format: aep.planning-md/3
id: story:approval-issuance-prepare-test-flake
kind: story
status: draft
title: approval_issuance prepare passivity test sees an extra store write in a full run
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Observed
`connectors-host` test
`local::owner::approval_issuance::tests::prepare_is_passive_and_digests_the_strict_input_and_current_policy`
(`crates/connectors-host/src/local/owner/approval_issuance/tests.rs:209`) failed once in a
full-workspace run on 2026-10-14: the observer's `PRAGMA data_version` read 3 where the test
expected 2. Run alone it passed 3 of 3 times.

The test opens a read-only observer on `metadata.sqlite3`, records `data_version`, calls
`prepare` and asserts the version is unchanged, to prove `prepare` writes nothing. A changed
`data_version` means some connection other than the observer committed to that database
between the two reads. That is either `prepare` writing (the property the test guards), or
another writer the fixture starts (for example a background task from `setup` or
`fake_issuer` that is still finishing). Which one is not established.

## Acceptance
- The writer that advances `data_version` in that window is identified from a reproduction
  (for example the test repeated under load), and the cause is recorded here.
- If `prepare` wrote, it no longer does, and the test keeps failing on any write by `prepare`.
- If the fixture wrote, the test waits for the fixture's writes to finish before its first
  read, so the assertion still fails on a write by `prepare` and on nothing else.
- The test passes 200 consecutive runs inside a full-workspace test run.
