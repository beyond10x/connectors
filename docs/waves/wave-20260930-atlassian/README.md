# Wave 2026-09-30: Confluence reads with a shared Atlassian auth profile

Skill version: aep implementing 0.18.0. Operator: "cleanup, then 2 documents: jira + confluence - but they can
share the auth profile" (clears decision-blocker:confluence-openapi-redistribution).

## Units

| story | serves | agents | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|---|
| story:catalog-confluence-reads | vision:independent-contract-adapters | aep:implementor, aep:adversary | impl/catalog-confluence-reads-v2 | `<managed-trees>`/wave0930-confluence | `<tree>`/target | `<wave-scratch>`/confluence | merged (0025717d8), target deleted |

Integration branch: `wave/20260930-atlassian` off `main` 90ed75a7b (release v0.17.0).

## Decisions

- One pinned document per provider; the Confluence v1 REST document if it carries all four reads.
- `atlassian.basic` shared by Jira and Confluence; `jira.basic` goes away (breaking, in the CHANGELOG).
- The held multi-source attempt (`wave0929b-confluence`, archived at the end of this wave) is reference only.

## Commits approval authorises

One unit commit, its merge, the closing store commit, the merge into `main` through a pull request once the
gate is green, then the release.

## Close

- Gate: `cargo run -p connectors-build -- gate --msrv` at `0025717d8`, `CONNECTORS_ESS` = ESS 0.45.0:
  `gate: all checks passed`, exit 0; 98 suites, 721 passed, 0 failed, 33 ignored; metadata conformance
  289 scenarios. A first run at the same commit stopped on `No space left on device` (other sessions'
  builds filled `/`) and tested nothing.
- Implemented: catalog-confluence-reads (v2 document alone, shared `atlassian.basic`).
- The earlier held attempt `wave0929b-confluence` is archived and removed.
