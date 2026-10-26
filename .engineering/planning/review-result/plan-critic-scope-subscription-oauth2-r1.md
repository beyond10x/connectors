---
format: aep.planning-md/3
id: review-result:plan-critic-scope-subscription-oauth2-r1
kind: review-result
status: active
title: 'Plan critic scope, round 1: subscription OAuth2 drafts'
relations:
- reviews: story:owner-rotating-refresh
- reviews: story:connections-token-issues-an-access-token
- reviews: architecture-decision-record:oauth2-grants-and-access-token-issue
revision: 1
---
verdict: needs-revision
reviewer: aep:plan-critic-scope (opus), round 1, 2026-10-25, over epic:subscription-oauth2-access, its decision record and 9 drafted stories

```findings
- file: .engineering/planning/story/owner-rotating-refresh.md
  line: 22
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the outcome limits the owner's refresh to declarations with `rotation: rotating`, but the domain also allows `non_rotating`, and ADR item 2 has the owner refresh any `oauth2_grant` connection. No drafted item says who refreshes a `non_rotating` `oauth2_grant` connection, and none says that one is refused. So the parent's \"hands a caller a currently valid access token\" is silently narrowed"
- file: .engineering/planning/story/connections-token-issues-an-access-token.md
  line: 48
  category: scope
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the optional `min_valid_seconds` input lets a caller force a refresh at will (`max(refresh_lead_seconds, min_valid_seconds)`), which the decision rules out (\"A caller cannot force a refresh\") and §4.1 forbids (\"it is never caller-invocable\"). The input should be dropped, or the decision changed"
- file: .engineering/planning/architecture-decision-record/oauth2-grants-and-access-token-issue.md
  line: 55
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the decision says \"§4.1's trigger text names issue-time expiry\" and gives that edit to this story and owner-rotating-refresh, but neither claims it. This story adds only a new \"access token issue\" section, and owner-rotating-refresh adds only the row and the binding name. The issue-time trigger belongs most naturally to this story"
```
