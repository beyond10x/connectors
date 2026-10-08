---
format: aep.planning-md/3
id: release-plan:connectors-v0340-stale-connections
kind: release-plan
status: implemented
title: 'Release 0.34.0: stale connections name their remedy, service help and completion'
relations:
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "active", at: "2026-10-08T10:58:26Z", actor: "human:timo", revision: 2}
- {from: "active", to: "implemented", at: "2026-10-08T13:22:00Z", actor: "human:timo", revision: 4}
---
## Outcome and authorization

Minor release 0.34.0 from origin/main `55bfe18133` (the merge of PR 133), under the operator's
rule "ready means ship" (2026-09-25) and the delegated release decision for this repository.
The tag namespace ends at v0.33.0, an ancestor of that base; recheck both before tagging.

Minor, not patch: a stale connection's refusals change their `next_action` (`retry_status` is
replaced by `revalidate_connection` or `create_connection`), and a new connection under the
configured revision of an existing instance id is admitted where 0.33.0 refused it. 0.34.0 can
record two keys in a stored connection binding (`refused_configuration_revision`,
`begun_at_instance_revision`); whether 0.33.0 reads such a store was not tested.

## Released scope (v0.33.0..55bfe18133)

| PR | Change |
|---|---|
| #133 | Wave 20261008a: a stale connection's refusal names the step that clears it, and a connection created under the configured revision moves its instance there; a changed provider authority or profile declaration still needs a new instance id. `help describe`, `help invoke`, `help serve` and bash completion for those commands. AEP 0.69.1. Due expiries in batches of at most 128 (from 32). |

Evidence: PR 133 repository gate https://github.com/beyond10x/connectors/actions/runs/37763487263
(success, head f0a174016f); measurements in the CHANGELOG 0.34.0 entry and
`specification:wave-20261008a-cli-audit` § Results.

## Not in this release

- The CLI audit's admission timeouts and saved revalidation `outcome_unknown` did not reproduce
  on 0.33.0 (`specification:wave-20261008a-cli-audit` § audit re-run); no change targets them.
- Slack advertises no operation because none is bound yet; that is the parity waves.
- Parse errors still carry only `cli_parse`: the declared-argument detail waits for an ESS
  release.
- The attempt id on the HTTP invoke path (`story:invoke-returns-attempt-id`) is the next wave.

## Released

- Tag `v0.34.0` (annotated, tagger `b10x-bot[bot]`) peels to `ca688db14d`, the merge of PR 134 on `main`; its tree equals the gated release commit `3bcea59dfd`.
- PR checks on `3bcea59dfd`: repository gate (27m22s), planning validate, b10x-docs-check, common / Security and privacy: all pass.
- Package gates on `3bcea59dfd`: clippy `-D warnings` and tests of all 14 workspace crates, 1473 passed, 0 failed.
- GitHub Release `v0.34.0` authored by `b10x-bot[bot]`, Latest, source archives only.
