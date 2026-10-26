---
format: aep.planning-md/3
id: review-result:plan-critic-acceptance-subscription-oauth2-r1
kind: review-result
status: active
title: 'Plan critic acceptance, round 1: subscription OAuth2 drafts'
relations:
- reviews: story:oauth2-grant-core
- reviews: story:oauth2-refresh-metadata-authority
- reviews: story:owner-rotating-refresh
- reviews: story:connect-runs-an-oauth2-grant
- reviews: story:connections-token-issues-an-access-token
- reviews: story:chatgpt-subscription-connector
- reviews: story:claude-subscription-connector
revision: 1
---
verdict: needs-revision
reviewer: aep:plan-critic-acceptance (opus), round 1, 2026-10-25, over epic:subscription-oauth2-access, its decision record and 9 drafted stories

```findings
- file: .engineering/planning/story/oauth2-grant-core.md
  line: 81
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance asks for the adapter boundary gate to pass with \"the crate depends on no adapter and no host crate\", but that gate only loops over a fixed list of eight adapter packages that does not include connectors-oauth2, so the check passes whether or not the crate depends on connectors-host; it should require the crate to be added to that list, or should name a cargo tree -p connectors-oauth2 check"
- file: .engineering/planning/story/oauth2-grant-core.md
  line: 26
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome says \"It holds no provider name, endpoint or client id\", and no acceptance item checks it (for example an rg over crates/connectors-oauth2/src for provider hosts and client ids that prints nothing)"
- file: .engineering/planning/story/oauth2-refresh-metadata-authority.md
  line: 61
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "\"kills the mutants whose site lies in the two domains, or the report names each survivor\" can never fail, because any mutation result passes once survivors are listed; it should name a kill threshold or say which survivors are allowed"
- file: .engineering/planning/story/owner-rotating-refresh.md
  line: 71
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the check \"rg -n oauth2_grant ...semantics.md prints the new row\" also passes when story:connect-runs-an-oauth2-grant adds oauth2_grant to the same file's flow table, so it cannot tell whether this story did its contract edit; it should name text only this story writes (the §4.1 binding)"
- file: .engineering/planning/story/connect-runs-an-oauth2-grant.md
  line: 72
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the CLI journeys are modelled on cli_journey.rs, where every follow-hook journey is #[ignore] and needs a disposable Secret Service, but the acceptance names no command that runs them; cargo test reports them as ignored, not failed, so it should name connectors-build ignored --required (which refuses missing prerequisites and zero-test runs)"
- file: .engineering/planning/story/connections-token-issues-an-access-token.md
  line: 66
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "token_without_reveal_prints_no_token searches only \"text and --format json output\", while the Design and the decision record's item 4 promise no token on standard error; the acceptance never searches stderr, with or without --reveal"
- file: .engineering/planning/story/connections-token-issues-an-access-token.md
  line: 57
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the contract edits this story owns (the verb row in contracts/cli/v1alpha1/semantics.md, which is hand-written rather than generated, and the auth contract's \"access token issue\" section) have no acceptance item, so the story can close without them"
- file: .engineering/planning/story/chatgpt-subscription-connector.md
  line: 72
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the three CLI journeys name no command that runs them, so \"passes\" cannot be told apart from \"ignored\"; it should name the connectors-build ignored --required run"
- file: .engineering/planning/story/claude-subscription-connector.md
  line: 65
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the three journeys name no command that runs them, so \"passes\" cannot be told apart from \"ignored\"; it should name the connectors-build ignored --required run"
```
