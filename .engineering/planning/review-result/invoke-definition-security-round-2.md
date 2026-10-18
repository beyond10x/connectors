---
format: aep.planning-md/3
id: review-result:invoke-definition-security-round-2
kind: review-result
status: active
title: Security review, round 2, on invoking a definition file
summary: |-
  needs-change

  Security review, round 2, of the round-1 fixes for story:invoke-definition-file at f9e181f3fd, run
  2026-10-18. Verdict: the round-1 cases are green; two introduced findings.

  Cases added: apps/connectors/tests/security_invoke_definition.rs
  (`security_a_dot_dot_file_stem_leaves_nothing_beside_the_cache_entries`,
  `security_an_unchanged_definition_reuses_its_entry_under_another_spelling_of_the_state_directory`)
  and crates/connectors-host/tests/security_definitions.rs
  (`security_an_entry_is_held_under_every_spelling_of_its_state_directory`), all red at f9e181f3fd.

  1. warning, introduced: with no `catalog`, a file stem of `..` still made the lowerer write 0644
     files beside the cache entries; the containment check ran only after the writes.
  2. warning, introduced: the argv pin and the cache-hit check compared path strings, so another
     spelling of the state directory refused every entry and deleted a live cache entry.

  Held: `remove_dir_all` cannot leave the definitions directory (no-follow walk, hex digest names);
  the containment check is correct once names are plain; symlinked state components are refused
  as a whole; an interrupt after recording leaves an entry that grants nothing.
relations:
- reviews: story:invoke-definition-file
revision: 1
---
# Security review, round 2, on invoking a definition file
