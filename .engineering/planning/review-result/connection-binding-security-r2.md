---
format: aep.planning-md/3
id: review-result:connection-binding-security-r2
kind: review-result
status: active
title: Security review, round 2, on launching for a description and writes
relations:
- reviews: story:connection-binding-excludes-operations
revision: 1
---
needs-change

Security review, round 2, of the delta on c53353d975 for story:connection-binding-excludes-operations.
Run 2026-10-18 on the uncommitted tree. Verdict: contract drift only; invariants 1 to 3 hold.

Cases added (test files only, `security2_` prefix) in
crates/connectors-host/src/local/registry/binding_scope_security_tests.rs,
adapters/catalog/tests/local_runtime/custody_journey.rs and
adapters/catalog/tests/local_runtime/revision_recovery_security.rs: a current usable connection admits
a launch; revoked, Stale, identity-refused and expired connections admit none; a no-digest binding
under a moved revision and a CA change admit none; another instance's connection admits none; a proof
issued before an executable-only change authorizes no write (`approval_refused`, nothing sent); a write
whose new executable answers another identity sends nothing; a revoked connection starts nothing for a
description; a description launch does not resume a stopped instance. Two were red: a changed provider
authority under an unchanged digest, and `operations describe` starting the owner against owner.md.

Held: the launch is `Task::Ensure { resume: false }`, reads no custody, validates nothing and is
digest-pinned; the owner re-runs `launch_admits`, so `launch: true` from a client gains nothing; the
proof is verified before the follow and `unchanged()` re-resolves after it.

Outcomes, recorded separately: the owner.md and semantics.md drift fixed, its case restated as
`security2_operations_describe_starts_no_owner_without_a_current_connection`; the provider-authority
note resolved by narrowing the contract to a moved credential-side digest, its case restated with the
digest moved.

```findings
[
{"file":"contracts/cli/v1alpha1/owner.md","line":9,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"operations describe and list now start the owner while owner.md and semantics.md still said passive commands only contact an existing owner"},
{"file":"crates/connectors-host/src/local/registry/observation.rs","line":176,"category":"contract-drift","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"launch_admits does not compare the provider authority, so the contract overclaimed; reachable only by an implementation whose credential digest omits its authority under an unchanged instance id"}
]
```
