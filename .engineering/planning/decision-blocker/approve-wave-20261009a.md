---
format: aep.planning-md/3
id: decision-blocker:approve-wave-20261009a
kind: decision-blocker
status: cleared
title: Nobody has approved wave 20261009a (Grafana, Jira issue reads, Slack discovery)
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-08T23:35:18Z", actor: "human:timo", revision: 2}
---
## Question

Approve wave 20261009a: story:parity-grafana-datasource-loki, story:parity-jira-issue-reads and story:parity-slack-discovery-reads, delivered on one integration branch (`wave/20261009a`) with one pull request, released when it merges.

## Units

| unit | surface | calls since 2026-09-09 |
|---|---|---|
| story:parity-grafana-datasource-loki | new crate `adapters/grafana` (`connectors-grafana`: `datasources.list` and the connection test, bearer service-account profile, configured identity probe `GET /api/datasources`), Cargo.toml members, Cargo.lock, crates/connectors-build/src/gate.rs, adapters/loki tests and docs (Loki through Grafana's data-source proxy), docs/fluxplane-plugin-parity.md | 394 |
| story:parity-jira-issue-reads | adapters/catalog/providers/jira (`getIssue`, `getCreateIssueMetaIssueTypes`, `findUsers`), catalog bundle and index | 398 |
| story:parity-slack-discovery-reads | adapters/catalog/providers/slack (`users.list`, `team.info`, `auth.test`, `emoji.list`; `search.messages` behind a user-token connection), catalog bundle and index, docs/catalog-slack.md | 512 (of which 236 + 93 already shipped in 0.38.0) |

Grafana takes the simplest working path. The Loki adapter's `base_url` already admits an HTTPS origin with a path prefix (`adapters/loki/spec/ess/domains/connection.yaml:27`), so `grafana.loki.query`, `grafana.loki.labels` and `grafana.loki.recent_logs` are served by a Loki connection whose `base_url` is Grafana's `api/datasources/proxy/uid/<uid>/` with a Grafana service-account bearer, proved by a fixture test with that prefix. `grafana.datasource.list` needs the new adapter. The mediated parent route of `adapters/grafana/design.md` (sealed UID, child degradation) is not built; it becomes a draft story.

The Jira and Slack units share the catalog bundle and index (generated); the integrator regenerates them once after both merge into the wave branch. No other file is shared.

## Build

One wave tree with one `target/`; units run one after the other in it, so the wave stays inside a 10G build budget (the 2026-10-07 wave tree measured 8.2G). Builds pause while / is under 20G free.

## Options

| option | what | cost |
|---|---|---|
| A | all three units | 1,304 calls; three units in sequence |
| B | Grafana and Jira issue reads | 792 calls; Slack discovery waits on its user-token question |
| C | Grafana alone | 394 calls |

Recommended: A. If search cannot be served by a second Slack catalog connection without a host change, the unit ships the other four operations and search stays partial in its story.

Not in this wave: story:parity-mysql-reads (704 calls). It adds a MySQL wire-protocol dependency and a new binding in `adapters/sql`, and it gets a wave of its own next.

## Decided

Option A, 2026-10-09: story:parity-grafana-datasource-loki, story:parity-jira-issue-reads and story:parity-slack-discovery-reads in wave 20261009a, one pull request, released when it merges.

- Spec first in each unit.
- The mediated parent route of `adapters/grafana/design.md` becomes a draft story; Loki through Grafana is a Loki connection on Grafana's data-source proxy.
- `search.messages` stays partial if serving it needs a host change; the pull request says so.
- Builds wait until other repositories' builds have released the disk (or / is at 50G free or more); until then the units are written and planned without cargo. The wave's build stays under 10G `du`, with `CARGO_BUILD_JOBS=8` while the 1-minute load is over 30.
