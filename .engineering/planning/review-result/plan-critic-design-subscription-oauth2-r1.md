---
format: aep.planning-md/3
id: review-result:plan-critic-design-subscription-oauth2-r1
kind: review-result
status: active
title: 'Plan critic design, round 1: subscription OAuth2 drafts'
relations:
- reviews: story:owner-rotating-refresh
- reviews: story:connect-runs-an-oauth2-grant
revision: 1
---
verdict: needs-revision
reviewer: aep:plan-critic-design (opus), round 1, 2026-10-25, over epic:subscription-oauth2-access, its decision record and 9 drafted stories

```findings
- file: .engineering/planning/story/owner-rotating-refresh.md
  line: 56
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance runs a refresh on an `oauth2_grant` connection, which needs the profile reader, the `oauth2` declaration block and the custody entry shape that story:connect-runs-an-oauth2-grant creates, and no depends_on edge runs from it to story:connect-runs-an-oauth2-grant, so `aep plan artifact waves` places it in wave 16, five waves before its producer (wave 21)"
- file: .engineering/planning/story/owner-rotating-refresh.md
  line: 50
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it claims the `oauth2_grant` row in `contracts/auth/acquisition/v1alpha1/semantics.md`, and story:connect-runs-an-oauth2-grant claims the same row, so the scheme's contract is split across two siblings; the row should belong to the story that introduces the scheme, and this one should keep only the §4.1 binding"
- file: .engineering/planning/story/connect-runs-an-oauth2-grant.md
  line: 31
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its body does not say how the `--device` path picks a profile's non-RFC device dialect, so the dialect trait that story:oauth2-grant-core defines and the only implementation of it, in story:chatgpt-subscription-connector, meet at a dispatch point no story owns or tests"
```
