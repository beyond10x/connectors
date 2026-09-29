---
format: aep.planning-md/3
id: review-result:adversary-gitlab-repository-reads-pass-1
kind: review-result
status: active
title: Adversary pass 1 on GitLab repository reads
relations:
- reviews: story:catalog-gitlab-repository-reads
revision: 1
---
unit: story:catalog-gitlab-repository-reads, uncommitted tree wave0929b-gitlab on 1d4df135b
verdict: CONFIRMED
cases: executed 22→27, red 1
origin: introduced 1 / pre-existing 0 / undecided 0
wrote-outside-worktree: scratch/adversary (empty)
needs-coordinator: none

Cases in adapters/catalog/tests/gitlab_repository_reads_adversary.rs: undeclared_parameter_is_refused_before_any_request,
time_filters_reach_the_transport_verbatim_and_bodies_are_unchanged, repository_reads_cannot_be_declared_as_writes,
planted_wrong_operation_id_sends_a_different_path (green); per_page_above_the_provider_cap_is_refused_before_any_request
(red: "`projects.list` sent per_page=101").

Coordinator routing: finding 1 fixed in docs/local-catalog-provider.md (the per_page cap stated beside the
stop rule); the refusal itself needs a selection-level bound and is story:catalog-selection-parameter-bounds,
which the red case is ignored against. Finding 2 (the paging stop condition lives in the test) kept as a note.

```findings
[
  {"file": "docs/local-catalog-provider.md", "line": 51, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "The documented stop rule (page shorter than per_page) ends a walk after page one when per_page exceeds GitLab's cap of 100, because the provider sends per_page=101 unrefused on all four repository reads."},
  {"file": "adapters/catalog/tests/local_runtime.rs", "line": 694, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The paging test's stop condition lives in the test itself, so it exercises the fixture and page passthrough rather than any provider paging behaviour."}
]
```
