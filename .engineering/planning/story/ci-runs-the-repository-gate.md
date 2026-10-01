---
format: aep.planning-md/3
id: story:ci-runs-the-repository-gate
kind: story
status: active
title: CI runs the repository gate on every pull request
relations:
- decomposes: epic:tech-debt-review-20260930
- serves: vision:independent-contract-adapters
scope:
- confidence: cited
  path: .github/workflows/rust-gate.yml
- confidence: cited
  path: docs/development.md
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T07:32:47Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-01T07:32:48Z", actor: "human:timo", revision: 4}
---
## Defect

No CI job builds or tests the Rust workspace. `.github/workflows/` holds `b10x-docs-bundle.yml`,
`b10x-docs-check.yml`, `b10x-docs-pages.yml`, `planning.yml` and `shared-gates.yml`, and the last
only calls `beyond10x/gates/.github/workflows/common.yml` (security and privacy).
`docs/development.md:20-21` says the repository "has no configured CI". A pull request merges with
the checks green while every Rust suite is unexamined; the release rule relies on a developer
having run `connectors-build gate --msrv` locally.

## Change

A new workflow, separate from the shared gate: on `pull_request` and on `push` to `main`, with
`permissions: contents: read`, no secrets and `persist-credentials: false`, it installs the pinned
ESS and AEP releases by checksum the way `planning.yml` installs AEP, and runs
`cargo run --locked -p connectors-build -- gate`. The shared gate keeps running on
`pull_request_target` and never reads candidate source; this workflow carries no credential.

## Scope

- `.github/workflows/rust-gate.yml` (new) — cited: the directory listing.
- `docs/development.md` — cited: line 20-21.

## Acceptance

- A pull request whose head breaks one Rust test shows the new check failing, and one with the
  test fixed shows it passing; both run ids recorded.
- The workflow declares `contents: read` only, references no secret, and its checkout sets
  `persist-credentials: false`.
- ESS and AEP are the versions in `crates/connectors-spec/toolchain.json` and
  `crates/connectors-build/aep-toolchain.json`, verified against the release's `SHA256SUMS`.
- `docs/development.md` names the workflow instead of saying there is no CI.
