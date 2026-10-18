---
format: aep.planning-md/3
id: review-result:definition-moved-authority-security-round-1
kind: review-result
status: active
title: Security review, round 1, on a definition moved to another host
summary: 'one confirmed pre-existing note: a left-behind definition connection was revocable through another connector''s catalog instance; fixed'
relations:
- reviews: task:definition-moved-authority-new-instance
revision: 1
---
# Security review, round 1, on a definition moved to another host

Security review, round 1, of the uncommitted change for task:definition-moved-authority-new-instance
on branch fix/definition-moved-authority (base `d123da5ed`), run 2026-10-18 by the
`aep:security-reviewer` agent. Verdict: one confirmed note, pre-existing; the six stated invariants
otherwise held under test.

Cases added:

- `crates/connectors-host/src/local/registry/upgrade_tests.rs`
  `a_left_behind_definition_connection_is_not_revocable_through_another_connector`: red before the
  fix (`Ok` where `NotFound` was expected), green after it.
- `crates/connectors-host/src/local/definitions/tests.rs`
  `security_review_a_moved_instance_is_shared_exactly_by_bindings_the_registry_admits_together`:
  across 128 bindings, two share a moved id exactly when the registry admits them under one
  instance id (same authority and `same_authentication`). Green.

1. note, pre-existing: `Registry::revoke_left_behind` scoped a left-behind connection by adapter id
   alone. Every invoked definition's entry has the adapter `catalog`, so a connection left under
   `forge.definition` could be revoked through the definition instance of another connector, given
   its exact reference and revision (revocation only; nothing read, copied or sent). Pre-existing
   at `d123da5ed`, but this change is the first to leave definition connections behind routinely.
   Fixed here: a connection left under a definition instance id is revocable only through an
   instance id `<connector>.<name>` of the same connector.

Checked and not faulted: no cross-host reuse (the registry checks authority and authentication at
listing and capture; the left-behind instance is configured by no entry and never offered); no use
under another authority; an unchanged definition keeps its instance id and cache key; no collision
or spoofing of the moved id (`is_instance`, `collides`, the read-back after recording); the
left-behind connection is untouched; every new step in `prepare_with` is followed by the interrupt
check.

```findings
[
{"file":"crates/connectors-host/src/local/registry/lifecycle.rs","line":413,"category":"integrity","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"revoke_left_behind was scoped by adapter id alone, so a connection a moved definition leaves under forge.definition was revocable through a catalog instance of another connector (upgrade_tests.rs a_left_behind_definition_connection_is_not_revocable_through_another_connector, red before the fix)"}
]
```
