---
format: aep.planning-md/3
id: release-plan:connectors-v0310-probe-waves
kind: release-plan
status: active
title: 'Release 0.31.0: refusal reasons, Retry-After, revalidate advice, clock floor and owner memory'
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-06T19:15:33Z", actor: "human:timo", revision: 2}
---
## Outcome and authorization

Prepare minor release 0.31.0 from origin/main 1f9e450. Operator rule "ready means ship"
(2026-09-25) and the operator's approval of the connector-probe waves on 2026-10-06 ("all
waves approves by me"). The tag namespace ends at v0.30.0, an ancestor of that base.
Recheck both before tagging. The conductor session confirmed no hold for open PR #124.

Minor, not patch: new CLI answer fields (`service_reason`, `retry_after_seconds`), a new
`next_action` value (`revalidate_connection`), a new catalog write (GitLab `issue.create`),
a new command (`connections launch`) and configuration format `connectors-local/3`.
Existing configurations, stored metadata and connections keep working; a store without
the new registry floor file refuses commands for at most 60 s after the upgrade
(CHANGELOG).

## Released scope (v0.30.0..1f9e450)

| PR | Change |
|---|---|
| #110 | Consumer launch: `connections launch`, configuration `connectors-local/3`. |
| #113 | ESS 0.53.0, AEP 0.68.0 and compatible crates.io updates. |
| #116 | An adapter identity mismatch during an upgrade names a new connection (tests pin the shipped behaviour). |
| #117 | GitLab catalog write `issue.create` through the forge selection (fixes #81). |
| #118 | A provider refusal carries the upstream reason, withheld when it may hold a credential, an address or invisible text. |
| #119 | The registry clock floor is recorded at most once a minute (part of #101). |
| #120 | The `datasource.feed/v1alpha1` contract family (contracts and specification only). |
| #121 | A read answered 429 honours `Retry-After`. |
| #122 | The owner keeps one malloc arena and no longer copies histories on open (part of #103). |
| #123 | An invoke on expired evidence advises `revalidate_connection`. |
| #111, #112, #114 | Planning only. |

## Not in this release

- #124 feed discovery and the catalog feed engine (open).
- #101: every command still opens and verifies the whole store
  (https://github.com/beyond10x/entity-runtime/issues/55).
- #103: the 506 MB owner peak target at 600 events (story:owner-memory-bounded stays active,
  upstream-blocker:er-model-record-copies, https://github.com/beyond10x/entity-runtime/issues/59).
- #105: operation schemas and results answered as JSON text (story:cli-json-answers-as-json,
  waiting for the ESS release that admits `Json` in the CLI generator).
- ESS 0.54.0 is released; connectors stays on 0.53.0 until the ESS release with the CLI
  `Json` primitive, adopted together with #105.
