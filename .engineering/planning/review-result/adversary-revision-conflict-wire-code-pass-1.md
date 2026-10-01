---
format: aep.planning-md/3
id: review-result:adversary-revision-conflict-wire-code-pass-1
kind: review-result
status: active
title: 'Adversary pass 1: revision-conflict-wire-code'
relations:
- reviews: story:revision-conflict-wire-code
revision: 1
---
unit: story:revision-conflict-wire-code; findings cover worktree wave1001c-revision at e7ed2deba plus one untracked test file
verdict: NEEDS-CHANGE (2 red cases; neither reachable in production today)
cases: executed 424→426, red 2
origin: introduced 2 / pre-existing 2 / undecided 0
wrote-outside-worktree: 3 paths (<scratch>/adv-suite.log, adv-cli.log, adv-metadata-entities.log)
needs-coordinator: whether semantics.md:483 is meant to hold for guarded writes and for the approval stores, or is narrowed to registry/CLI-local writes

Cases (apps/connectors/tests/revision_conflict_adversary.rs), both red on first run:
- a_guarded_write_revision_conflict_reports_what_the_semantics_row_says: left: ("admission", "retry_status") / right: ("publication", "retry_explicitly")
- approval_stores_do_not_fold_a_revision_conflict_into_an_unreadable_store: approval_keys: a revision conflict becomes metadata_unavailable / left: MetadataUnavailable
Suite: `cargo test -p connectors -p connectors-host --no-fail-fast` exit 101, 424 passed, 2 failed. cli --check and
metadata-entities --check exit 0.

Not broken: every CLI mapper naming the variant maps it (owner_failure, host_failure, registry_failure); adapters and
MCP have no mapping site; ConcurrentRevision comes only from execute_batch NotCommitted/Store (er.rs:3874), so
retry_explicitly is honest there; the stand-in owner test runs the real binary with real decoding; no consumer
matches metadata_unavailable for this case; binding.json consistent (29/29).

```findings
- file: crates/connectors-host/src/local/owner/mutation.rs
  line: 60
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: a guarded write's host RevisionConflict reports admission/retry_status while the new semantics.md:483 row promises publication/retry_explicitly, and the row contradicts the guarded-write rule at semantics.md:584
- file: crates/connectors-host/src/local/approval_keys.rs
  line: 36
  category: acceptance
  severity: warning
  verdict: INFEASIBLE
  origin: pre-existing
  message: approval_keys and approval_policy (and the private host_error folds in audit, mutations, approvals/spend) still turn ConcurrentRevision into MetadataUnavailable, so the CLI would answer metadata_unavailable for a revision conflict there
- file: crates/connectors-host/src/local/registry.rs
  line: 265
  category: judgement
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: exhausting the eight observation retries still answers MetadataUnavailable rather than ConcurrentRevision; no reachable path found under the lifecycle lock
- file: contracts/cli/v1alpha1/semantics.md
  line: 483
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: 'the phrase "the store''s recorded revision moved" (revision_conflict) sits beside "stale revision" (lifecycle_conflict, :486), so one word names two codes'
```
