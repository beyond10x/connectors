---
format: aep.planning-md/3
id: review-result:invoke-definition-adversary-round-1
kind: review-result
status: active
title: Adversarial review, round 1, on invoking a definition file
summary: |-
  needs-change

  Adversarial review, round 1, of the green change for story:invoke-definition-file on branch
  unit/invoke-definition-file at 98e0c677fa, run 2026-10-18. Verdict: one introduced defect, three
  judgement notes; selectors, the command-line rewrite, the collision rule, the overlay, the native
  configuration and the lowering inputs held.

  Case added: apps/connectors/tests/adversary_invoke_definition.rs
  `adversary_an_interrupt_while_the_definition_is_prepared_ends_the_command`, red at 98e0c677fa
  (exit 2 `invalid_configuration` instead of exit 130 `interrupted`).

  1. warning, introduced: SIGINT and SIGTERM during preparation (lowering, pinning, describing) were
     caught by the installed handler and never checked, so the command went on instead of ending
     as interrupted.
  2. note: the 64-entry cap was undocumented and no command forgets an entry or prunes the cache.
  3. note: a concurrent run of another file with the same connector id refused this run as
     `definition_instance_configured` although nothing is configured.
  4. note: a cache hit only checked that `native.json` exists; lowered files were not flushed before
     the entry was published.

  Held: `--definition` is exclusive with the other selectors in every option form; kept arguments
  are re-emitted as `--long=value`; the collision rule covers instance, awaiting profile, alias and
  instance id and is re-applied on every read; entries never reach the operator's file; the native
  configuration matches what the provider derives; the cache key covers every file `lower()` reads;
  contract, ESS, help and guide agree with the code.
relations:
- reviews: story:invoke-definition-file
- supersedes: review-result:invoke-definition-adversary-r1
revision: 1
---
# Adversarial review, round 1, on invoking a definition file
