---
format: aep.planning-md/3
id: review-result:adversary-terminal-sigint-test-flake-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: terminal-sigint-test-flake'
relations:
- reviews: story:terminal-sigint-test-flake
revision: 1
---
unit: story:terminal-sigint-test-flake, working tree at 9e8dceacf plus my uncommitted cases in tests.rs
verdict: CONFIRMED. The fix holds, but the unit's suite stays green under two wrong fixes.
cases: executed 308→311, red 0 (on the tree; all 3 are red on both mutants)
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 (deleted: wave-20261001b/sigint/scratch/adv-mutant/)
needs-coordinator: no lease taken; a mutant build briefly overwrote this worktree's test binary (rebuilt after touching tests.rs)

Cases appended to crates/connectors-host/src/local/protected/tests.rs (~:375), green on the tree, red on both mutants:
- adversary_empty_read_without_interrupt_waits_for_the_next_input (Ok(1), 2 reads)
- adversary_empty_read_with_nothing_further_blocks_in_poll_until_the_deadline (Err(Timeout), 1 read, 350–1000 ms at a 400 ms deadline)
- adversary_empty_read_then_writer_closes_reports_end_of_input (Ok(0), 2 reads)
Mutant A (protected.rs:204): WouldBlock => Err(Interrupted) — red Err(Interrupted). Mutant B (:202-204): no poll, busy loop — red 180539 / 529021 / 9528 reads. The unit's own two SIGINT tests stay ok on both mutants (6 passed; 3 failed).

Suite: `cargo test -p connectors-host --lib` 287 passed; 0 failed; 24 ignored, EXIT=0; fmt and clippy clean.

Not broken: no real fd type keeps reporting readable while every read returns EAGAIN; check runs before every retry and the poll timeout is min(remaining, 1000); SIGINT between check and poll seen within one poll interval; echo restore via Terminal::drop unchanged; 24/200 → 0/200 confirmed from logs.

```findings
- file: crates/connectors-host/src/local/protected/tests.rs
  line: 276
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the forced-interleaving test and the SIGINT test stay green when read_ready maps WouldBlock straight to Interrupted or spins on read without poll; the three adversary cases kill both mutants
- file: crates/connectors-host/src/local/protected/tests.rs
  line: 347
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the owner test accepts the child's exit 0, and a --exact filter that matches nothing exits 0, so renaming flush_fixture makes the test pass without running it
```
