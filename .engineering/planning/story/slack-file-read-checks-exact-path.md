---
format: aep.planning-md/3
id: story:slack-file-read-checks-exact-path
kind: story
status: draft
title: Slack file-read tests compare the request path exactly
tags:
- from-finding
relations:
- informed_by: review-result:wave-a-http-empty-query-adversary-1
revision: 1
---
## Defect

Found by the adversary of `story:http-empty-query-no-trailing-question-mark`
(`review-result:wave-a-http-empty-query-adversary-1`), pre-existing: two checks in
`adapters/catalog/tests/slack.rs` (lines 1183 and 2435 on `impl/http-empty-query-no-trailing-question-mark`
`ecfa7d465`) strip a trailing `?` with `trim_end_matches` before comparing the request path, so they
accept both `/files-pri/...` and `/files-pri/...?` and would not catch the trailing `?` coming back.

## Acceptance

- Both checks compare the request path by exact equality; they pass on the fixed host and would fail
  if `ScopedHttp` sent a trailing `?` again.
