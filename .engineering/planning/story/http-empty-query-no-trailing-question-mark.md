---
format: aep.planning-md/3
id: story:http-empty-query-no-trailing-question-mark
kind: story
status: active
title: A provider request with no query parameters is sent without a trailing ?
relations:
- serves: vision:independent-contract-adapters
- decomposes: epic:zero-open-issues-and-store-defects
scope:
- confidence: cited
  path: adapters/catalog/tests/confluence_binding.rs
- confidence: cited
  path: adapters/catalog/tests/confluence_v1.rs
- confidence: cited
  path: adapters/catalog/tests/gateway_prefix.rs
- confidence: cited
  path: adapters/catalog/tests/gateway_prefix_adversary.rs
- confidence: cited
  path: adapters/catalog/tests/gitlab_ci_project_writes.rs
- confidence: cited
  path: adapters/catalog/tests/gitlab_commit_reads_adversary_pass2.rs
- confidence: cited
  path: adapters/catalog/tests/gitlab_issue_create.rs
- confidence: cited
  path: adapters/catalog/tests/gitlab_mr_variants.rs
- confidence: cited
  path: adapters/catalog/tests/gitlab_mr_writes.rs
- confidence: cited
  path: adapters/catalog/tests/gitlab_tags_releases.rs
- confidence: cited
  path: adapters/catalog/tests/google_calendar.rs
- confidence: cited
  path: adapters/catalog/tests/google_drive.rs
- confidence: cited
  path: adapters/catalog/tests/google_gmail.rs
- confidence: cited
  path: adapters/catalog/tests/google_slides.rs
- confidence: cited
  path: adapters/catalog/tests/hubspot.rs
- confidence: cited
  path: adapters/catalog/tests/jira.rs
- confidence: cited
  path: adapters/catalog/tests/jira_issue_writes.rs
- confidence: cited
  path: adapters/catalog/tests/jira_transition_run.rs
- confidence: cited
  path: adapters/catalog/tests/local_runtime.rs
- confidence: cited
  path: adapters/catalog/tests/openrouter.rs
- confidence: cited
  path: adapters/catalog/tests/opsgenie.rs
- confidence: cited
  path: adapters/catalog/tests/runpod.rs
- confidence: cited
  path: adapters/catalog/tests/runpod_adversary.rs
- confidence: cited
  path: adapters/catalog/tests/slack.rs
- confidence: cited
  path: adapters/catalog/tests/zendesk.rs
- confidence: cited
  path: adapters/grafana/tests/local_runtime.rs
- confidence: cited
  path: adapters/kubernetes/tests/local_runtime.rs
- confidence: cited
  path: adapters/tavily/tests/protocol.rs
- confidence: cited
  path: crates/connectors-host/src/http.rs
- confidence: cited
  path: crates/connectors-host/tests/fixed_header_adversary.rs
- confidence: cited
  path: crates/connectors-host/tests/http.rs
- confidence: cited
  path: crates/connectors-host/tests/http_service_account_token.rs
- confidence: cited
  path: crates/connectors-host/tests/http_write.rs
revision: 38
transitions:
- {from: "draft", to: "proposed", at: "2026-11-09T21:58:06Z", actor: "human:timo", revision: 2, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
- {from: "proposed", to: "active", at: "2026-11-09T21:58:06Z", actor: "human:timo", revision: 3, executor: "agent:connectors-zero-issues", correlation: "connectors-zero-issues"}
---
## Source

Found by the implementor of story:catalog-google-drive-reads on 2026-09-30: a GET with no query
parameters goes out as `/drive/v3/changes/startPageToken?`. `ScopedHttp` calls
`url.query_pairs_mut()` unconditionally (`crates/connectors-host/src/http.rs:239` at b50ffba79),
which sets an empty query. Pre-existing; every catalog provider is affected. Google, GitLab, Jira and
Confluence accept the form (recorded fixtures pass), so nothing is known to fail.

## Acceptance

- `crates/connectors-host/tests/http.rs` gains `empty_query_sends_no_question_mark`: a request with
  no query pairs reaches the fixture with a path and no `?`.
- The recorded-fixture expectations that encode the trailing `?`
  (`adapters/catalog/tests/google_drive.rs`) are updated to the form without it.

## Scope

Landed on, from `git diff 9069c43a4...impl/http-empty-query-no-trailing-question-mark` (merged into wave/zero-issues at a64f6396b); the earlier scoper lines marked inferred are replaced by this list. Generated reference pages are left out.

- `adapters/catalog/tests/confluence_binding.rs`
- `adapters/catalog/tests/confluence_v1.rs`
- `adapters/catalog/tests/gateway_prefix_adversary.rs`
- `adapters/catalog/tests/gateway_prefix.rs`
- `adapters/catalog/tests/gitlab_ci_project_writes.rs`
- `adapters/catalog/tests/gitlab_commit_reads_adversary_pass2.rs`
- `adapters/catalog/tests/gitlab_issue_create.rs`
- `adapters/catalog/tests/gitlab_mr_variants.rs`
- `adapters/catalog/tests/gitlab_mr_writes.rs`
- `adapters/catalog/tests/gitlab_tags_releases.rs`
- `adapters/catalog/tests/google_calendar.rs`
- `adapters/catalog/tests/google_drive.rs`
- `adapters/catalog/tests/google_gmail.rs`
- `adapters/catalog/tests/google_slides.rs`
- `adapters/catalog/tests/hubspot.rs`
- `adapters/catalog/tests/jira_issue_writes.rs`
- `adapters/catalog/tests/jira.rs`
- `adapters/catalog/tests/jira_transition_run.rs`
- `adapters/catalog/tests/local_runtime.rs`
- `adapters/catalog/tests/openrouter.rs`
- `adapters/catalog/tests/opsgenie.rs`
- `adapters/catalog/tests/runpod_adversary.rs`
- `adapters/catalog/tests/runpod.rs`
- `adapters/catalog/tests/slack.rs`
- `adapters/catalog/tests/zendesk.rs`
- `adapters/grafana/tests/local_runtime.rs`
- `adapters/kubernetes/tests/local_runtime.rs`
- `adapters/tavily/tests/protocol.rs`
- `crates/connectors-host/src/http.rs`
- `crates/connectors-host/tests/fixed_header_adversary.rs`
- `crates/connectors-host/tests/http.rs`
- `crates/connectors-host/tests/http_service_account_token.rs`
- `crates/connectors-host/tests/http_write.rs`
