---
format: aep.planning-md/3
id: review-result:wave-20261019-night-b3-adversary
kind: review-result
status: active
title: Adversary pass on the Jira issue writes
relations:
- reviews: story:parity-jira-issue-create-and-comment
- reviews: story:parity-jira-issue-writes
- reviews: story:parity-jira-create-meta-per-type
revision: 1
---
**Verdict: one CONFIRMED note, pre-existing; no change to the unit.**

Adversary (opus) on `7ae4ed552`, wave 20261019-night. 15 cases in
`adapters/catalog/tests/jira_issue_writes_adversary.rs` (`ce86a36bb`), 14 pass: withheld
parameters unreachable, stamp and key comparisons of the edit, delete and link guards, the fixed
comment document, the visibility enum, 409 as conflict, delete applied only on a 404 after it.
Red: a comment whose `/body/content` is JSON null is sent, because the closed-body engine admits
null for a required untyped key (`adapters/catalog/src/closed.rs:366`), which
`adversary_catalog_body_types.rs:113` asserts. The case is `#[ignore]` and is
story:required-body-key-refuses-null.
