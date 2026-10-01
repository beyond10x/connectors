# Wave 2026-10-01 (b): CLI and catalog follow-ups after 0.21.0

Skill version: aep implementing 0.18.0. Status: approved by the operator 2026-10-01 ("i approve the wave").

## Units

| story | serves | scope | agents | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|---|---|
| story:terminal-sigint-test-flake | vision:independent-contract-adapters | cited | aep:implementor, aep:adversary | impl/terminal-sigint-test-flake | `<managed-trees>`/wave1001b-sigint | `<tree>`/target | `<wave-scratch>`/sigint | merged (pass 1 recorded; coordinator-verified correction) |
| story:catalog-gitlab-commit-reads | vision:independent-contract-adapters | cited (one inferred test file) | aep:implementor, aep:adversary | impl/catalog-gitlab-commit-reads | `<managed-trees>`/wave1001b-gitlab | `<tree>`/target | `<wave-scratch>`/gitlab | implemented fcc47d696; adversary pass 1 red (1 introduced, 2 pre-existing → 2 stories); correction round |
| story:socket-path-limit-in-long-checkouts | vision:independent-contract-adapters | cited for `apps/connectors/tests`; two host test modules inferred | aep:implementor, aep:adversary | impl/socket-path-limit-in-long-checkouts | `<managed-trees>`/wave1001b-socket | `<tree>`/target | `<wave-scratch>`/socket | merged (pass 1 recorded; docs-only correction 2a8a10c20, no pass 2) |
| story:cli-spec-mapping-fixes | vision:independent-contract-adapters | cited `ess/domains/cli.yaml`; generated outputs inferred | aep:implementor, aep:adversary | impl/cli-spec-mapping-fixes | `<managed-trees>`/wave1001b-spec | `<tree>`/target | `<wave-scratch>`/spec | merged (passes 1 and 2 recorded; head 9d3827389) |

Integration branch: `wave/20261001b-cli-followups` off `main` 852b9b1a52.

## Selection

`aep plan artifact waves --kind story --status draft --format json` (aep 0.68.0) on `main` 2d2e9d42c places all
four in wave 1 with no collision between them. Wave 20260930c (Google) merged as PR #61 and wave 20261001-techdebt
as PR #66, so no open branch changes these files; PRs #67 and #68 change only planning-store files.

The CLI stories follow in later verb waves, in this order, because each pair collides on
`apps/connectors/src/local.rs`, `ess/domains/cli.yaml` or the generated CLI contract:

| verb wave | story |
|---|---|
| 2 | revision-conflict-wire-code |
| 3 | configuration-change-error, spec-models-operation-admission |
| 4 | expired-evidence-invoke-advises-revalidate |
| 5 | setup-initialises-missing-state |

Line numbers in `cli-spec-mapping-fixes` after `ess/domains/cli.yaml:187` moved by about 9 lines with PR #61;
the implementor re-derives them.

## Pre-flight

- `/` 25G free (2026-10-01). One full gate wrote 11G of `target/` on 2026-09-30. Units gate package-scoped.
  Dispatch two units first (sigint, gitlab); the other two start when free disk is above 15G.
- sccache at its 30 GiB cap.
- The full gate runs from a short-path checkout of the integration commit until the socket-path unit merges.
- Gate env: `CONNECTORS_ESS` = ESS 0.45.0, `CONNECTORS_AEP` = AEP 0.65.0 (the pins). CI now runs the gate on
  every pull request (PR #66).

## Commits approval authorises

One commit per unit, the merges into `wave/20261001b-cli-followups`, the opening and closing store commits,
the push of the integration branch and its pull request through `b10x-gates`, and the merge into `main` once
the gate is green. No tag and no release.

## Alignment with the Google session (2026-10-01)

- Open there: PR #69 (`crates/connectors-build/src/{docs,gate}.rs`, `docs/development.md`, `website/README.md`),
  then release 0.21.1 (`Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, `README.md`, status page).
- socket-path-limit-in-long-checkouts reaches the owner socket through the state directory's descriptor in the
  tests and does not edit `crates/connectors-build/src/gate.rs`.
- cli-spec-mapping-fixes adds no reference from a domain the website example model reads to `connectors.cli`.
- Before the closing gate, the integration branch merges `main` (0.21.1).

## Paused 2026-10-01 14:05 (operator: free CPU)

- All agents and the 0.21.1 release gate stopped; no process of this wave left running.
- Release 0.21.1: commit 2806bc677 on local branch `rel/0.21.1-gated` (worktree rel-0-21-1, not pushed); gate
  from the short-path clone `~/.cache/c211` (build dir `~/.cache/b10x-target/c211`) was killed mid-compile and
  has no result. PR #69 is merged (f905839ad).

## Resumed 2026-10-01 15:20

Load 2.3, 53G free. 0.21.1 gate rerun in `~/.cache/c211`; both adversaries re-dispatched; sigint resumed by a fresh
implementor; socket unit dispatched. Agents use `CARGO_BUILD_JOBS=4`.

## Release 0.21.1 (handed over by the Google session)

- PR #69 merged as f905839ad; release PR #70 merged as 37f19c7ce, tree equal to the gated tree 2806bc677.
- Gate at 2806bc677 from a short-path clone: all checks passed, 136 suites, 1098 passed, 0 failed, 43 ignored;
  `npm run build` in `website/` exit 0.
- Tag v0.21.1 by b10x-bot[bot] peels to 37f19c7ce on main; release page by b10x-bot[bot], Latest.
- Recorded on story:website-build-examples-ess-enum (artifact and test_result evidence).
- `origin/main` merged into the wave at 4891c038e; `docs/development.md` conflict resolved by keeping both changes.

## Close

- Gate: `RUSTC_WRAPPER= cargo run -p connectors-build -- gate --msrv` at 4891c038e from the managed worktree
  `wave1001b-gate` (own build directory), `CONNECTORS_ESS` = ESS 0.45.0, `CONNECTORS_AEP` = AEP 0.65.0:
  `gate: all checks passed`, exit 0; 142 suites, 1135 passed, 0 failed, 43 ignored; metadata conformance
  289 scenarios. No `SUN_LEN` in the log, which is the socket story's acceptance.
- Implemented: terminal-sigint-test-flake, catalog-gitlab-commit-reads, socket-path-limit-in-long-checkouts,
  cli-spec-mapping-fixes.
- Adversary passes: gitlab 2, spec 2, socket 1 (docs-only correction), sigint 1 (correction verified by the
  coordinator). The store kept one review_outcome record where two identical outcomes were recorded against one
  review, so its counts are lower than the findings: gitlab 3+2 findings, spec 3+4, socket 1, sigint 2.
- Filed: catalog-selection-excludes-parameters, catalog-validates-enum-and-date-time,
  gate-temporary-root-and-compiler-wrapper, service-failure-carries-upstream-reason (from a consumer's Confluence
  401, cause: token scopes).
- Paused 14:05–15:20 by the operator to free CPU; all agents and the release gate were stopped and resumed.
- Not done in this wave: two stale "ESS 0.40" comments (`ess/domains/cli.yaml:659`, `ess/domains/clock.yaml:27`).
