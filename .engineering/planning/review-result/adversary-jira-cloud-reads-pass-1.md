---
format: aep.planning-md/3
id: review-result:adversary-jira-cloud-reads-pass-1
kind: review-result
status: active
title: Adversary pass 1 on Jira Cloud reads
relations:
- reviews: story:catalog-jira-cloud-reads
revision: 1
---
unit: story:catalog-jira-cloud-reads, uncommitted tree wave0929b-jira on 987ab2bd1
verdict: CONFIRMED
cases: executed 32→36, red 1
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: scratch/adversary/suite.log; scratch copies deleted
needs-coordinator: the planted edits the brief asked for were made on scratch copies, not in the tree

Cases in adapters/catalog/tests/jira_cloud_reads_adversary.rs: guide_end_condition_for_search_covers_the_null_token_the_pinned_document_names
(red: the guide named only an absent nextPageToken), reserved_characters_in_jql_and_token_reach_the_wire_form_encoded,
an_undeclared_parameter_is_refused_before_any_request, the_basic_credential_is_in_no_argv_environ_or_failure (green).
Source-hash step on scratch copies: a corrupted gzip and a rebuilt gzip fail; a changed raw file alone passes that
step and is caught by jira.rs and bundle_drift instead.

Coordinator routing: finding 1 fixed in docs/catalog-jira.md (end condition "absent or null", never send a null
token back) and the jira.rs guide expectation; finding 2 fixed (bundle_drift.rs SOURCES carries a literal auth
profile per provider, gitlab.pat and jira.basic); finding 3 kept as a note (one page per call is the design).
The Apache 2.0 text was added as adapters/atlassian/upstream/LICENSE. `cargo test -p connectors-catalog-provider`
exit 0 afterwards.

```findings
[
  {"file": "docs/catalog-jira.md", "line": 47, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "The issues.search end condition names only an absent nextPageToken, while the pinned document's response schema says the last page's token will be null."},
  {"file": "adapters/catalog/tests/bundle_drift.rs", "line": 35, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Rebuilding each bundle with the auth profile read from that same committed bundle drops the former gitlab.pat literal, so a bundle and index rebuilt together with a different gitlab profile still pass."},
  {"file": "adapters/catalog/tests/jira.rs", "line": 496, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The paging tests compute the stop condition in test code from fixture bodies, so only parameter forwarding, not the end condition, can fail for a provider defect."}
]
```
