---
format: aep.planning-md/3
id: epic:slack-complete-web-api
kind: epic
status: draft
title: The complete Slack Web API under four token profiles, generated; limits are per-instance policy
summary: Every Slack Web API method under user, bot, app-level and configuration tokens, generated from pinned sources
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

Connectors offers the complete Slack Web API. The catalog describes each method as Slack documents
it, under every token type that may call it: user (`xoxp`, profile `slack.user`), bot (`xoxb`,
`slack.bot`), app-level (`xapp`, `slack.app`) and configuration (`xoxe`, `slack.config`). The
operation lists are generated from pinned sources, not written by hand. What one instance may do
is per-instance policy: its allow-list, approvals for writes, and a check of each operation's
scopes against the token's scopes.

## Source

Timo, 2026-11-23, relayed by the session `bf-runbook`: "what I actually want from connector and
the Slack integration is just full API access. Why isn't that there yet?", and on the answer that
the user profile has no writes by design: "sounds like some business rules leaked into core
design". Direction as relayed: the catalog describes the API as it is; rules such as "no user
writes" become per-instance policy; xapp Socket Mode is its own story, out of the first wave; the
xoxe token-rotation rules are marked unverified.

## Today (origin/main 45276d755c4)

- Request shapes come from Slack's archived OpenAPI v2 (174 paths; `slackapi/slack-api-specs`
  commit `bc08db4`, archived since 2021-09), pinned by SHA-256 in
  `adapters/slack/upstream/README.md`.
- Selections are hand-written YAML per operation in `connectors-definitions`
  (`connectors/slack.yaml`, `connectors/slack-user.yaml`; fixture copies under
  `crates/connectors-catalog/tests/fixtures/definitions/connectors/`). Bot: 18 operations
  (`adapters/catalog/providers/slack/operations.json`). User: 5, all reads
  (`user-operations.json`).
- "No writes on a user token" is catalog data plus a test,
  `adapters/catalog/tests/adversary_credential_pass1.rs:290-296`. No engine code names
  `slack.user` (git grep over `*.rs`: tests only).
- Per-instance policy that exists: `[adapters.<x>.permissions] profiles, operations` in the
  configuration, and an approval proof for every `effect: write` operation at admission
  (`approval_required`, `apps/connectors/src/local/invoke.rs:1085`). No scope check against the
  token exists.
- `slackapi/node-slack-sdk` `packages/web-api/src/methods.ts` on its default branch, read
  2026-11-23: 303 `bindApiCall` entries (grep count).

## Stories

1. `story:slack-sdk-upstream-import`: methods and inputs from node-slack-sdk at a pinned commit.
2. `story:slack-method-token-scope-table`: per method, token types, scopes, read or write.
3. `story:definitions-default-write-guard`: a default guard for generated writes.
4. `story:slack-four-profile-definition-generated`: one Slack definition with four profiles.
5. `story:admission-checks-token-scopes`: admission refuses an operation whose scope the token
   lacks.
6. `story:slack-user-writes-by-policy`: the user profile carries writes; the limit is policy.
7. `story:slack-socket-mode-stream`: the xapp Socket Mode stream. Out of the first wave.

## Out of scope

- A raw pass-through of any URL or method name not in the generated catalog: every operation
  stays declared, generated from the pinned sources.
- Slack APIs outside the Web API (SCIM, Audit Logs, Legal Holds, Discovery).
- Releases.

## Noun

The token's granted scopes are a new fact about a connection (story 5). The ESS model in
`ess/` declares no entity or field for them; story 5 models it first (guardrail 7) and its
relation to the connection is `UNMAPPED:` until then.
