---
format: aep.planning-md/3
id: epic:tech-debt-review-20260930
kind: epic
status: draft
title: Debt found by the 2026-09-30 repository review
summary: Fix or carry every verification, pinning, store and branch gap the review found.
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

The debt found by the repository review of 2026-09-30 is either fixed or carried by a story that
names what is missing. The review read `origin/main` `74fac9f1f` and the planning store; the full
findings are in the operator's report of that day.

## Findings and where each is carried

| Finding | Source | Carried by |
|---|---|---|
| CI runs no Rust build, test, format or lint | `.github/workflows/` holds docs-bundle, docs-check, docs-pages, planning, shared-gates; `docs/development.md:20-21` | `story:ci-runs-the-repository-gate` |
| 35 `#[ignore]` tests and no command or job that runs them | `git grep '#\[ignore'`; last recorded `--ignored` run `review-result:adversary-settlement-pass-2-20260912` | `story:ignored-suites-have-a-runner` |
| ESS pinned 0.45.0 and AEP 0.65.0 while 0.48.0 and 0.67.0 are released; a machine set up with `b10x upgrade` cannot run the gate without fetching old releases by hand | `crates/connectors-spec/toolchain.json`, `crates/connectors-build/aep-toolchain.json`; `gh release list` | `story:toolchain-pins-newest-release-20261001` |
| Two release plans still `active` for shipped tags v0.15.0 and v0.2.0; a draft story that duplicates an implemented one | store | `story:planning-store-hygiene-20261001` |
| 32 unmerged remote branches from 2026-09-01..23 | `git for-each-ref refs/remotes/origin` | `story:retire-unmerged-remote-branches` |
| A rate-limited read is returned as a refusal; no `Retry-After` handling | `docs/local-catalog-provider.md` Limits, `docs/catalog-*.md` Limits | `story:catalog-honours-retry-after` |
| Jira, Confluence and HubSpot are verified against local fixtures only | the three guides' Limits sections | `story:live-reads-jira-confluence-hubspot` |
| The gate fails from a managed worktree (SUN_LEN) | `crates/connectors-build/src/gate.rs:7-16` | `story:socket-path-limit-in-long-checkouts` (existing) |
| The website build fails at `connectors-build examples` | `npm run build` | `story:website-build-examples-ess-enum` (existing) |
| Load-sensitive tests | | `story:host-gate-load-sensitivity-w3`, `story:terminal-sigint-test-flake` (existing) |
| A GET-only read model | `adapters/catalog/src/lib.rs:239-247` | `story:catalog-post-reads` (existing) |

## Out of scope

Consumers on the predecessor (devcenter pins `connectors` 0.7.0): a change in another repository,
its own requested scope per `AGENTS.md`. The open decision blockers on Helm execution and MCP
process and connection ownership: they wait on the operator, not on work.
