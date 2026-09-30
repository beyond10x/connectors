# Wave 2026-09-30: Confluence reads with a shared Atlassian auth profile

Skill version: aep implementing 0.18.0. Operator: "cleanup, then 2 documents: jira + confluence - but they can
share the auth profile" (clears decision-blocker:confluence-openapi-redistribution).

## Units

| story | serves | agents | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|---|
| story:catalog-confluence-reads | vision:independent-contract-adapters | aep:implementor, aep:adversary | impl/catalog-confluence-reads-v2 | `<managed-trees>`/wave0930-confluence | `<tree>`/target | `<wave-scratch>`/confluence | dispatched |

Integration branch: `wave/20260930-atlassian` off `main` 90ed75a7b (release v0.17.0).

## Decisions

- One pinned document per provider; the Confluence v1 REST document if it carries all four reads.
- `atlassian.basic` shared by Jira and Confluence; `jira.basic` goes away (breaking, in the CHANGELOG).
- The held multi-source attempt (`wave0929b-confluence`, archived at the end of this wave) is reference only.

## Commits approval authorises

One unit commit, its merge, the closing store commit, the merge into `main` through a pull request once the
gate is green, then the release.
