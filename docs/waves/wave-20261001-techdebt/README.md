# Wave 2026-10-01: CI runs the repository gate; shipped plans closed

Skill version: aep implementing 0.19.1. Operator: "file tech debt findings as stories into the
planning store, then select first wave to fix it". Status: approved by the operator 2026-10-01 ("approved, keep going").

## Units

| story | serves | agents | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|---|
| story:ci-runs-the-repository-gate | vision:independent-contract-adapters | aep:implementor, aep:adversary | impl/ci-runs-the-repository-gate | `<managed-trees>`/wave1001-ci (id wave1001-ci) | `<tree>`/target | `<tree>`/.local/tmp/unit | merged (c4d2ba96b), target deleted |
| story:planning-store-hygiene-20261001 | vision:independent-contract-adapters | coordinator (store writes are the coordinator's) | wave/20261001-techdebt | `<managed-trees>`/connectors-techdebt | — | — | implemented (0cd17f8c9) |

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

## Close

- Gate: `cargo run -p connectors-build -- gate --msrv` at `82658e5e5` from a short-path clone, ESS 0.45.0,
  AEP 0.65.0: `gate: all checks passed`, exit 0; 136 suites, 1096 passed, 0 failed, 43 ignored. The
  first run at the same commit failed one test,
  `google_calendar_gmail_adversary_pass2::an_attachment_under_the_named_limit_is_refused_and_the_texts_state_the_real_ceiling`
  ("a 3144704-byte part failed: Unavailable"), which then passed 10 of 10 alone; filed as
  `story:gmail-attachment-read-flaky-under-load`.
- `story:ci-runs-the-repository-gate`: implemented. GitHub run 36835676252 on probe PR #63 (one
  assertion broken on purpose) failed at `adapters/catalog/tests/hubspot.rs:133`; PR #63 closed unmerged
  and its branch deleted. Run 36839187046 on this wave's PR passed in 18m30s; runner disk 82G free,
  `target/` 2.9G. The adversary's concurrency finding was fixed in `9f0882e62` and re-checked green.
- `story:planning-store-hygiene-20261001`: implemented. `release-plan:connectors-0-15-0` implemented;
  `release-plan:gitlab-v020` superseded by `release-plan:connectors-v080`; the duplicate replay-flake
  story had already been archived in `7e5015cf1`.
- Required since 2026-10-01 12:10 (operator: "sure gate is mandatory"): ruleset 23511229 lists
  `repository gate` beside `common / Security and privacy` and `planning validate`; strict mode on.
- `main` gained wave `20260930c` during this wave; its merge brought one conflict in
  `docs/development.md`, resolved by keeping both the SUN_LEN paragraph and the CI paragraph.
