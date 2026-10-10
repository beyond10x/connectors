---
format: aep.planning-md/3
id: decision-blocker:wave-20261010d-scope
kind: decision-blocker
status: cleared
title: Which parity units wave 20261010d delivers
relations:
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-10T09:55:31Z", actor: "human:timo", revision: 2}
---
## Question

Which parity units wave 20261010d delivers, by call count on `docs/fluxplane-plugin-parity.md`.

## Options

| option | what | cost |
|---|---|---|
| A | story:parity-gitlab-ci-writes (U15, 36 calls) and story:parity-gitlab-project-writes (U16, 36), one unit, both change the GitLab selection | one tree, catalog only, about 6G |
| B | A plus story:parity-kubernetes-reads (U18, 17, native adapter) and story:parity-confluence-cql-search (U20, 8, a new pinned Confluence v1 OpenAPI source) | one tree, units built in turn, at most 10G |
| C | B without the Confluence unit | as B, about 8G |

Held, not offered: story:parity-jira-issue-writes (326) and story:parity-slack-message-writes (162) wait on work outside this wave; story:parity-homer-sip-reads (33) needs a reviewed login-exchange auth profile.

Recommended: B.

## Decided

Option B, 2026-10-10: the GitLab CI and project writes as one unit with an adversary review, then the Kubernetes reads, then the Confluence CQL search, in one tree built one unit at a time with `cargo clean` between units, at most 10G on disk. If no Confluence v1 OpenAPI document can be pinned, that unit stops and the wave ships without it. Package gates run locally and the full gate in the pull request's CI. The wave is released as 0.44.0 after it merges on green CI.
