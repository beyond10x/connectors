---
format: aep.planning-md/3
id: review-result:invoke-definition-security-round-1
kind: review-result
status: active
title: Security review, round 1, on invoking a definition file
summary: |-
  needs-change

  Security review, round 1, of the green change for story:invoke-definition-file on branch
  unit/invoke-definition-file at 98e0c677fa, run 2026-10-18. Verdict: two introduced defects, two
  notes on owner-uid-only writes; nothing reaches the provider before the operator answers yes.

  Cases added: apps/connectors/tests/security_invoke_definition.rs
  (`security_a_catalog_name_never_places_lowered_files_outside_its_cache_entry`,
  `security_an_interrupt_while_a_definition_is_prepared_is_exit_130`) and
  crates/connectors-host/tests/security_definitions.rs
  (`security_an_entry_naming_another_executable_or_argv_is_not_held`), all three red at 98e0c677fa.

  1. warning, introduced: `catalog: '..'` (the lowerer's `valid_id` admits it) made the lowering
     write `connector.json` and the selection, mode 0644, beside the cache entries in
     `<state>/definitions/`, shared across definitions, and the entry's native configuration named
     a path outside its entry.
  2. warning, introduced: an interrupt during preparation was dropped (as the adversary's 1).
  3. note: a held entry could name any executable and argv; reachable only by an owner-uid write to
     an owner-only file.
  4. note: a cache hit did not check that the digest directory is private and not a link; reachable
     only by an owner-uid write.

  Held: prompts use only the foreground controlling terminal with settings restored; the
  entries file is admitted as a private file; definition paths cannot leave the definitions root and
  the lowerer reads only the private copy; the operator's file is never written and a configured
  instance wins; an authority change answers `lifecycle_conflict` and sends nothing; envelopes carry
  no secret or path.
relations:
- reviews: story:invoke-definition-file
- supersedes: review-result:invoke-definition-security-r1
revision: 1
---
# Security review, round 1, on invoking a definition file
