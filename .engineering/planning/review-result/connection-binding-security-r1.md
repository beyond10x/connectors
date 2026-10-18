---
format: aep.planning-md/3
id: review-result:connection-binding-security-r1
kind: review-result
status: active
title: Security review, round 1, on the credential binding change
relations:
- reviews: story:connection-binding-excludes-operations
revision: 1
---
needs-change

Security review of the green change for story:connection-binding-excludes-operations, on the
uncommitted tree of branch unit/connection-binding-excludes-operations (base 82a681bc4), run
2026-10-18. Verdict: the acceptance is not met; no credential-routing gap was found.

Cases added (test files only): crates/connectors-host/src/local/registry/binding_scope_security_tests.rs,
adapters/catalog/tests/credential_revision_security.rs and
adapters/catalog/tests/local_runtime/revision_recovery_security.rs, seven cases. Six are green; the
acceptance case `security_the_added_read_succeeds_after_a_selection_only_change_without_revalidation`
is red with `description_unavailable` / `refresh_description`.

Invariants checked and held: every credential-routing input moves the credential-side digest (ten
inputs: a host's credential flag, a host origin, the API host and port, header and bearer, request
prefix, identity path, scopes probe, minimum scopes, CA bytes), and a selection change alone does not;
the host binds the configured entry's digest and refuses a child that prints another; an
executable-only refusal records nothing; a binding with no digest keeps the configuration-revision
rule and a full ER replay keeps both fields; the approval subject still binds the configuration
revision and the executable selection, and an approval target refuses an unvalidated executable
change; a consumer launch rechecks the configuration revision after reading the credential.

Outcomes, recorded separately: finding 1 escalated to the coordinator as a decision (the same as the
adversary's A); finding 3 fixed (the automatic validation now refuses unless its own capture
classifies the change as executable-only); finding 2 accepted as D12's operation side and named in
the report.

```findings
[
{"file":"adapters/catalog/tests/local_runtime/revision_recovery_security.rs","line":32,"category":"acceptance","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"after a selection-only change operations describe answers description_unavailable, so the added read cannot run without connections revalidate, contrary to the story's acceptance"},
{"file":"adapters/catalog/src/local.rs","line":1280,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a download path_prefix on a credential-carrying admitted host is operation-side, so a selection edit widens which paths on that host receive the credential without revalidation"},
{"file":"crates/connectors-host/src/local/owner/supervisor.rs","line":1121,"category":"concurrency","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"the automatic validation called capture_revalidation, which admits any upgradable binding, in a transaction separate from the ExecutableChanged classification and without re-checking it"}
]
```
