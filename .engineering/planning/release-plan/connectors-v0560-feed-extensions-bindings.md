---
format: aep.planning-md/3
id: release-plan:connectors-v0560-feed-extensions-bindings
kind: release-plan
status: implemented
title: 'Release 0.56.0: catalog bindings, feed engine extensions, Jira and Confluence feeds over fixtures'
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "active", at: "2026-10-22T05:25:27Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":1}}}
- {from: "active", to: "implemented", at: "2026-10-22T07:48:05Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":2}}}
---
## Outcome

Minor release 0.56.0 of wave 20261021 (`approval-record:wave-20261021`):
- One catalog instance binds the API its token reaches (`bindings`), and the refusal tells `insufficient_scope` from `invalid_credential`.
- The catalog feed engine reads the list shapes Jira and Confluence answer with, including `utc_offsets` for a time query read in an unknown zone.
- The Jira issue feed and the Confluence REST v1 page feed are specified and held over test fixtures.
- Fixes from three adversary passes and one security review, two of them to the shipped GitLab feed.
- The ESS pin moves to 0.67.0.

Minor rather than patch, because:
- there is a new native configuration member (`bindings`) and new feed declaration syntax;
- the connection results carry new members (`binding`, `rebound`);
- answers change: an unresumable feed watermark is `stale_cursor`, and a listing 404 or 410 after the lookup is `not_found`.

## Scope

Merged in this release:

| story | status | what remains |
|---|---|---|
| story:catalog-feed-engine-extensions | active | — |
| story:connection-follows-configuration-upgrade | active | — |
| story:confluence-token-kind-binding | active | shipped declarations carry no `bindings` |
| story:jira-feed-binding | active | `upstream-blocker:definitions-notation-lacks-feed-extensions` |
| story:confluence-feed-binding | active | `upstream-blocker:definitions-notation-lacks-feed-extensions` |

Moving the stories is left to the operator.

Out of the wave: story:sustained-live-read-loop (`credential-blocker:live-read-loop-no-configured-connections`). Already in 0.54.0: story:connection-lease-holds-unattended.

## Evidence

- The repository gate with MSRV ran on 2026-10-22 through `b10x-build-slot`: `connectors-build gate --msrv` with ESS 0.67.0 and aep 0.71.3, on the release tree (wave head `d0c32e43c` plus the version, CHANGELOG, README, getting-started and connector page edits).
  - 28 steps, all exit 0.
  - 2944 tests passed, 0 failed, 146 ignored.
  - 12 Rust 1.88.0 steps and 1 Rust 1.91 step among them.
  - The gate printed "gate: all checks passed".
- `target/debug/connectors --version` printed `connectors 0.56.0`, and `connectors-docs generate` wrote 0 of 135 generated files.
- Reviews:
  - review-result:feed-extensions-security-20261022
  - review-result:wave-20261021-adversary-u0-u4
  - review-result:wave-20261021-adversary-u5
  - review-result:wave-20261021-adversary-u6
- Each review has a `review_outcome` of fixed on the stories it reviewed.

## Limitations

These are the limitations the CHANGELOG's 0.56.0 section states:
- No shipped selection carries the Jira or Confluence feed.
- No shipped declaration carries `bindings`.
- The bindings and both feeds are verified against local fixtures only.
- A feed's per-millisecond bounds, and the re-read window, leave it `unavailable` at about 1,000 changes.
- 100 or more GitLab merge requests changed in one millisecond make a restarting consumer loop.
