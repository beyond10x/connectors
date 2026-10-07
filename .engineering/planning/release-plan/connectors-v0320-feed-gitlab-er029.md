---
format: aep.planning-md/3
id: release-plan:connectors-v0320-feed-gitlab-er029
kind: release-plan
status: active
title: 'Release 0.32.0: JSON answers, the GitLab feed binding and Entity Runtime 0.29.0'
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-07T16:05:49Z", actor: "human:timo", revision: 2}
---
## Outcome and authorization

Minor release 0.32.0 from origin/main `0ce1e22e4`, under the operator's rule "ready means ship"
(2026-09-25) and the delegated release decision for this repository. The tag namespace ends at
v0.31.0, an ancestor of that base; recheck both before tagging.

Minor, not patch: `--output json` answer fields change type in place (breaking for a caller that
decoded them twice), a catalog feed declaration requires `capabilities` (breaking for selection
files that declare a feed), a new provider binding (GitLab feed) and new runtime pins. Instances
on the shipped GitLab selection get a new configuration revision; the CHANGELOG states the
migration (`connections revalidate`). Stores this version writes stay readable by 0.31.0.

## Released scope (v0.31.0..0ce1e22e4)

| PR | Change |
|---|---|
| #127 | ESS 0.55.0; `operations invoke`, `operations describe`, `adapters describe` and the compatibility `describe` answer JSON values instead of JSON text (#105). |
| #126 | Wave 20261006d's planning: feed engine extensions, Swagger 2 projection, feed profile capabilities. |
| #128 | Wave 20261007a: feed profile capabilities; the GitLab feed binding (profile `gitlab-merge-requests/1`, readable through a saved connection); a read through the owner answers its result as a JSON value; planning (decided blockers, MCP caller decision, branch retirement, fluxplane-plugin parity matrix and 25 parity stories). |
| #129 | Wave 20261007b: Entity Runtime 0.29.0 and Eventlog 0.8.0 (each record held once; durable open checkpoints not enabled). |

## Not in this release

- #101: every command still opens and verifies the whole store; enabling durable open
  checkpoints (one-way for Entity Runtime 0.28.0 and earlier) is
  `story:metadata-invoke-cost-flat-in-store-size`.
- #103: the owner peak target is not re-measured on the new runtime (`story:owner-memory-bounded`).
- The GitLab feed read from a running GitLab (`task:gitlab-feed-sandbox-replay`).
- The Jira, Confluence and Slack feed bindings, and the fluxplane-plugin parity waves.
