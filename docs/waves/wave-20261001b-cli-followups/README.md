# Wave 2026-10-01 (b): CLI and catalog follow-ups after 0.21.0

Skill version: aep implementing 0.18.0. Status: approved by the operator 2026-10-01 ("i approve the wave").

## Units

| story | serves | scope | agents | branch | worktree | build dir | scratch | stage |
|---|---|---|---|---|---|---|---|---|
| story:terminal-sigint-test-flake | vision:independent-contract-adapters | cited | aep:implementor, aep:adversary | impl/terminal-sigint-test-flake | `<managed-trees>`/wave1001b-sigint | `<b10x-target>`/connectors-1001b-sigint | `<wave-scratch>`/sigint | dispatched |
| story:catalog-gitlab-commit-reads | vision:independent-contract-adapters | cited (one inferred test file) | aep:implementor, aep:adversary | impl/catalog-gitlab-commit-reads | `<managed-trees>`/wave1001b-gitlab | `<b10x-target>`/connectors-1001b-gitlab | `<wave-scratch>`/gitlab | dispatched |
| story:socket-path-limit-in-long-checkouts | vision:independent-contract-adapters | cited for `apps/connectors/tests`; two host test modules inferred | aep:implementor, aep:adversary | impl/socket-path-limit-in-long-checkouts | `<managed-trees>`/wave1001b-socket | `<b10x-target>`/connectors-1001b-socket | `<wave-scratch>`/socket | dispatched |
| story:cli-spec-mapping-fixes | vision:independent-contract-adapters | cited `ess/domains/cli.yaml`; generated outputs inferred | aep:implementor, aep:adversary | impl/cli-spec-mapping-fixes | `<managed-trees>`/wave1001b-spec | `<b10x-target>`/connectors-1001b-spec | `<wave-scratch>`/spec | dispatched |

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
