---
format: aep.planning-md/3
id: specification:wave-20261007a-feed-gitlab
kind: specification
status: approved
title: 'Wave 20261007a: feed profile capabilities and the GitLab feed binding'
relations:
- informed_by: epic:generic-datasource-feeds
- serves: vision:independent-contract-adapters
revision: 4
transitions:
- {from: "draft", to: "in_review", at: "2026-10-07T00:10:05Z", actor: "human:timo", revision: 2}
- {from: "in_review", to: "approved", at: "2026-10-07T00:10:06Z", actor: "human:timo", revision: 3}
---
## Wave 20261007a: feed profile capabilities and the GitLab feed binding

Opened 2026-10-07, `aep:implementing` 0.20.1 wave mode. Goal: the first feed binding a
consumer can read through a saved connection (cortex `story:feed-source` waits on a
Connectors release with one).

**Approval:** the operator approved every wave up front on 2026-10-05 ("I approve all waves
upfront and now. do not ask for permission, you orchestrate this") and set the delivery shape
on 2026-10-07: find the next waves through planning, deliver each through one integration
branch, and open one pull request from it, so CI runs one full gate per wave.

## Integration

| | |
|---|---|
| integration branch | `wave/20261007a` |
| integration tree | managed `conn-plan-20261007` |
| carries before the units | the planning commits: two decided blockers landed, the inbound MCP caller decision, the merged wave-20261006d planning, the feed stories' revised scope, the remote-branch retirement record |
| pull request | one, `wave/20261007a` into `main`, after the full gate passes on the integration tree |

## Units

They ran one after the other: U2 forked from the integration branch after U1 merged, because
both edit `crates/connectors-build/src/catalog_feed_conformance.rs`, `adapters/catalog/src/feed.rs`,
`adapters/catalog/spec/ess/domains/feed.yaml` and `contracts/catalog/v1alpha1/semantics.md`. U3
was found by U2 and ran in U2's tree after it, by the same implementor.

| unit | story | branch | worktree (managed id) | head | stage |
|---|---|---|---|---|---|
| U1 | `story:feed-profile-capabilities` | `unit/feed-profile-capabilities` | `conn-u1-fpc` (removed; archive kept) | `1e7567cfe` | merged into `wave/20261007a` at `2ac242fb6` after two adversary passes |
| U2 | `story:gitlab-feed-binding` | `unit/gitlab-feed-binding-20261007` | `conn-u2-gitlab` | `38f0523db`, `3d50a0740` | two adversary passes; final correction |
| U3 | `story:owner-read-answers-json-value` | same branch as U2 | `conn-u2-gitlab` | `cab270ebe`, `3d50a0740` | two adversary passes (with U2) |

Build dirs are each tree's own `target/`; scratch is `<tree>/.local/tmp/<unit>`. Dispatch:
`aep:implementor` per unit, `aep:adversary` two passes per unit.

## Selection

`aep plan artifact waves --kind story --format json` on the integration tree, 2026-10-07:
`story:feed-profile-capabilities` in wave 3, `story:gitlab-feed-binding` in wave 12. The
collisions it reports for the two units, verbatim:

```
{"a":"story:catalog-api-base-gateway-prefix","b":"story:gitlab-feed-binding","path":"adapters/catalog/tests/gateway_prefix.rs","confidence":"cited"}
{"a":"story:catalog-basic-auth-profile","b":"story:gitlab-feed-binding","path":"adapters/catalog/tests/local_runtime.rs","confidence":"inferred"}
{"a":"story:catalog-basic-auth-profile","b":"story:gitlab-feed-binding","path":"adapters/catalog/tests/local_runtime/cli_journey.rs","confidence":"inferred"}
{"a":"story:catalog-cli-journeys","b":"story:gitlab-feed-binding","path":"adapters/catalog/tests/local_runtime.rs","confidence":"inferred"}
{"a":"story:catalog-client-credentials-profile","b":"story:gitlab-feed-binding","path":"adapters/catalog/tests/local_runtime/oauth2_refresh.rs","confidence":"cited"}
{"a":"story:catalog-discovery-projection","b":"story:gitlab-feed-binding","path":"crates/connectors-build/src/main.rs","confidence":"inferred"}
{"a":"story:catalog-feed-engine","b":"story:feed-profile-capabilities","path":"contracts/catalog/v1alpha1/semantics.md","confidence":"inferred"}
{"a":"story:catalog-feed-engine","b":"story:gitlab-feed-binding","path":"contracts/catalog/v1alpha1/semantics.md","confidence":"inferred"}
```

Of the stories named opposite the two units, five are `implemented`;
`story:catalog-client-credentials-profile` is `proposed`, although the scheme it asks for
shipped in 0.29.0 (`CHANGELOG.md`, 0.29.0, `oauth2_client_credentials`). None is in this wave. The verb's unassessed list holds 38 stories store-wide, none of them in this wave. Both
units' scope is cited for their primary files and inferred for the journey routes
(`story:gitlab-feed-binding` `## Scope`).

Left out: `story:catalog-feed-engine-extensions`, `story:jira-feed-binding`,
`story:confluence-feed-binding`, `story:catalog-swagger2-projection`, `story:catalog-slack-reads`
and `story:slack-feed-binding`. One binding is enough to unblock the consumer, and GitLab needs the
fewest engine changes (two of the six shapes).

## Pre-flight

| check | value |
|---|---|
| free disk on `/` | 18,451 MiB (`df -m /`, 2026-10-07); builds wait until it is above 20 GiB, a floor set for every repository on this machine |
| one measured build | 8,139 MiB: `gate --msrv` in one connectors tree (`worktree finish --discard-cache`, 2026-10-07) |
| units at once | 1 |
| build cache | none shared; each tree builds into its own `target/` |
| ESS | 0.55.0 on `main` after pull request 127 |

## Commits this approval covers

The planning commits already on `wave/20261007a`; the merge of `main` into it; one commit per
unit through `b10x-gates bot`; the merge of each unit into `wave/20261007a`; the closing
planning-store commit; the one pull request into `main` and its merge. Not a release.
