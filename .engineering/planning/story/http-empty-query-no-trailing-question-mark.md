---
format: aep.planning-md/3
id: story:http-empty-query-no-trailing-question-mark
kind: story
status: draft
title: A provider request with no query parameters is sent without a trailing ?
relations:
- serves: vision:independent-contract-adapters
revision: 1
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
