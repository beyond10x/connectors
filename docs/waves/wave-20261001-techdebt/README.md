# Wave 2026-10-01: CI runs the repository gate; shipped plans closed

Skill version: aep implementing 0.19.1. Operator: "file tech debt findings as stories into the
planning store, then select first wave to fix it". Status: approved by the operator 2026-10-01 ("approved, keep going").

## Units

| story | serves | agents | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|---|
| story:ci-runs-the-repository-gate | vision:independent-contract-adapters | aep:implementor, aep:adversary | impl/ci-runs-the-repository-gate | `<managed-trees>`/wave1001-ci (id wave1001-ci) | `<tree>`/target | `<tree>`/.local/tmp/unit | dispatched |
| story:planning-store-hygiene-20261001 | vision:independent-contract-adapters | coordinator (store writes are the coordinator's) | wave/20261001-techdebt | `<managed-trees>`/connectors-techdebt | — | — | in progress |

Integration branch: `wave/20261001-techdebt` off `main` 5d861f9a1 (PR #62).

Scope: both cited. `ci-runs-the-repository-gate`: `.github/workflows/rust-gate.yml` (new),
`docs/development.md`. `planning-store-hygiene-20261001`: two release-plan files and one story file.

## Selection

`aep plan artifact waves --kind story --status draft` (aep 0.67.0) places both in wave 1 with no
collision between them. Left out of this wave, with the reason:

| story | reason |
|---|---|
| catalog-honours-retry-after | `adapters/catalog/src/lib.rs` and the catalog guides change on the unmerged wave `20260930c` (Google, 234 files); the verb cannot see branches |
| socket-path-limit-in-long-checkouts | `apps/connectors/tests/absent_operation.rs`, `failed_connect.rs`, `failed_connect_adversary.rs` change on wave `20260930c`; no typed scope |
| toolchain-pins-newest-release-20261001 | `Cargo.lock` changes on wave `20260930c`; collides with `ci-runs-the-repository-gate` on `docs/development.md` |
| ignored-suites-have-a-runner | `crates/connectors-build/src/main.rs` changes on wave `20260930c`; collides on `docs/development.md` |
| website-build-examples-ess-enum | its likely fix, `ess/domains/cli.yaml`, changes on wave `20260930c`; no typed scope |
| retire-unmerged-remote-branches | blocked: `decision-blocker:v1-remote-branch-retirement` |
| live-reads-jira-confluence-hubspot | blocked: `credential-blocker:sandbox-accounts-jira-confluence-hubspot` |

## Pre-flight

- `/` 17G free (2026-10-01). One full gate wrote 8.8G of `target/` on 2026-09-30; one unit, one gate.
- The gate cannot run from a managed worktree (SUN_LEN); it runs from a short-path clone of the
  integration commit, as on 2026-09-30.
- The CI unit's acceptance needs two GitHub runs: one on a throwaway branch with a test broken on
  purpose (closed, never merged) and one on the wave PR.

## Commits approval authorises

One unit commit, its merge into the integration branch, one store commit for the hygiene moves and
the closing store commit, a push of the throwaway CI branch and its PR closed unmerged, and the merge
of the integration branch into `main` through a pull request once the gate is green. No release.
