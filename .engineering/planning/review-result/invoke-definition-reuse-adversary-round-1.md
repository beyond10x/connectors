---
format: aep.planning-md/3
id: review-result:invoke-definition-reuse-adversary-round-1
kind: review-result
status: active
title: Adversarial review, round 1, on reusing a stored credential
summary: 'needs-change: a source revoked while its copy validated was still copied (blocker); an over-long listing hid the fresh connect (note)'
relations:
- reviews: story:invoke-definition-reuse-credential
revision: 1
---
# Adversarial review, round 1, on reusing a stored credential

Adversarial review, round 1, of the green change for story:invoke-definition-reuse-credential on
branch unit/invoke-definition-reuse-credential at 4a9d3cb01c, run 2026-10-18. Verdict:
needs-change, one introduced blocker and one introduced note.

Case added: adapters/catalog/tests/local_runtime/adversary_reuse_race.rs
`cli_journey::invoke_definition::adversary_reuse_race::adversary_reuse_a_source_revoked_while_its_copy_validates_is_not_published`,
red at 4a9d3cb01c on 2 of 2 runs: the source was revoked while its copy validated and the copy
was published as a ready connection.

1. blocker, introduced: after the source's read use was released, the owner prepared and
   published the copy without looking at the source again; a source revoked, made Stale or
   reconnected during validation still had its copy published.
2. note, introduced: a candidate listing over the 4096 bytes `protected::ask` shows was refused
   as the no-terminal `not_found` and never offered the fresh connect. Not reached in practice.
3. note (no row): a failure while listing candidates aborted the command instead of asking the
   plain connect question.

Held: the candidate rule (other connector, legacy entry, other adapter, other authority, other
profile, own instance, Stale, revoked, expired evidence, moved entry); a revision change between
listing and capture is `lifecycle_conflict`; a revoke between capture and dispatch releases the
use; the read use is cancelled on a failed custody read or configuration check; the acquisition is
failed on every error after allocation; refusal codes match owner.md; no material in the question;
OAuth profiles are filtered in the CLI and the owner.

```findings
[
{"file":"crates/connectors-host/src/local/owner/transport.rs","line":2294,"category":"concurrency","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"a source revoked, marked Stale or reconnected after its read use is dispatched still has its copy published, because nothing after release_read looks at the source again"},
{"file":"apps/connectors/src/local/definition.rs","line":755,"category":"boundary","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"a candidate question over ask's 4096-byte limit is refused as the no-terminal not_found and never offers the fresh connect"}
]
```
