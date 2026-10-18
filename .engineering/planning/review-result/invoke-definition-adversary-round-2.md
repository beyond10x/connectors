---
format: aep.planning-md/3
id: review-result:invoke-definition-adversary-round-2
kind: review-result
status: active
title: Adversarial review, round 2, on invoking a definition file
summary: |-
  needs-change

  Adversarial review, round 2, of the round-1 fixes for story:invoke-definition-file at f9e181f3fd,
  run 2026-10-18. Verdict: the interrupt fix holds; the new cache and entry checks compared the state
  path as text.

  Cases added: crates/connectors-host/src/local/definitions/tests.rs
  `adversary_an_entry_is_held_whichever_spelling_names_the_state_directory` and
  apps/connectors/tests/adversary_invoke_definition_r2.rs
  `adversary_a_cache_entry_is_reused_whichever_spelling_names_the_state_directory`, both red at
  f9e181f3fd.

  1. warning, introduced: `admitted` and `published` compared the state path as text, so another
     admitted spelling of the same directory (`/./`, `//`) refused every held entry and deleted and
     lowered a live cache entry again.
  2. note: refusals of record, read-back, connection and description returned before the next
     interrupt check.

  Held: interrupt checks after the cache and pinning steps; the plain-name rule and the containment
  check; `sync_tree`; the `revision_conflict` mapping; two concurrent runs of one file do not
  conflict.
relations:
- reviews: story:invoke-definition-file
revision: 1
---
# Adversarial review, round 2, on invoking a definition file
