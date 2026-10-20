---
format: aep.planning-md/3
id: story:definitions-pin-covers-catalog
kind: story
status: draft
title: The gate is red when an in-tree catalog connector has no definition at the pinned connectors-definitions tag
relations:
- decomposes: epic:catalog-connectors-only-in-definitions
- serves: vision:independent-contract-adapters
- depends_on: story:definitions-install
revision: 3
---
## Outcome

- This repository pins one connectors-definitions tag in a committed lock in the format of
  `story:definitions-install`. A step that runs before cargo, in CI and locally, fetches and
  digest-checks that release into a cache outside the working tree.
- A shared test-support function opens the installed pinned release and lowers one named
  connector with the reference lowerer into a temporary directory, writing what
  `connectors-build definition` writes (bundle, index, selection, declaration;
  `crates/connectors-build/src/main.rs:117-137`). `story:definition-tests-read-pinned-release`,
  `story:catalog-tests-read-pinned-release-gitlab-engine` and
  `story:catalog-tests-read-pinned-release-providers` read connector data only through it.
- This story adds the dependency on that function to every consuming crate manifest: a
  dev-dependency in `adapters/catalog/Cargo.toml`, `crates/connectors-catalog/Cargo.toml` and
  `apps/connectors/Cargo.toml`, and whatever `crates/connectors-build/Cargo.toml` and
  `crates/connectors-docs/Cargo.toml` need for their gate code, with the matching `Cargo.lock`
  entries. None of the three test stories touches a manifest or `Cargo.lock`.
- A check, the first caller of that function, fails naming each selection under
  `adapters/catalog/providers/` that has no definition at the pinned tag or whose definition does
  not lower. Epic step 2.

## Context

- 14 directories under `adapters/catalog/providers/` hold 15 selection files; `slack/` holds
  `operations.json` and `user-operations.json` (tree at 9201b1177).
- A selection maps to a definition name by `definition_name` in
  `crates/connectors-catalog/src/import.rs:542-549`: `operations.json` is `<catalog>`,
  `<x>-operations.json` is `<catalog>-<x>` (so `slack-user`). connectors-definitions
  `connectors/README.md` states the same rule.
- The gate runs `cargo build` and `cargo test --workspace --locked --offline`
  (`crates/connectors-build/src/gate.rs:149-150`); CI runs `cargo fetch --locked`, then the gate
  (`.github/workflows/rust-gate.yml:56,62-63`). Tests therefore read a release fetched before
  cargo runs and never fetch; a test that finds none fails naming the fetch command (inferred).
  The fetch is not a gate or `connectors-build` subcommand: `connectors-build` depends on
  `connectors-catalog-provider` (`crates/connectors-build/Cargo.toml:11`), whose `build.rs` embeds
  this release under option C of `decision-blocker:catalog-first-run-source`
  (`story:instances-resolve-through-locked-index`). `pages.yml`, `macos-host.yml` and
  `windows-host.yml` run tests or generators that
  the later stories point at the release, so they fetch it first too (inferred from
  `.github/workflows/pages.yml:51-52`, `macos-host.yml:43`, `windows-host.yml:51`).
- The crate that holds the test-support function is chosen at implementation. A dev-dependency
  back onto a crate it depends on (for example `connectors-catalog`, which holds the lowerer) is a
  cycle Cargo admits for tests but builds that crate twice; the choice says how it avoids or
  accepts that (inferred).
- "Exists" is read as "exists and lowers without refusal", so a definition that is present but
  unusable is red too. Byte equality with the in-tree files is held later, by
  `crates/connectors-catalog/tests/definition_import.rs` once it reads the release.

## Domain relations

- In-tree selection to definition: one-to-one by name, 15 to 15. Inferred from
  `crates/connectors-catalog/src/import.rs:542-549` (`definition_name`) and `:561-575`
  (`shipped_selections`); no `ess/1` document declares it.
- This repository to its pinned release: one pin, one tag, in the ADR D5 lock format applied to
  the repository itself (inferred).

## Scope

- a committed lock at the repository root (inferred; path chosen at implementation)
- the test-support crate (inferred)
- `adapters/catalog/Cargo.toml`, `crates/connectors-catalog/Cargo.toml`,
  `crates/connectors-build/Cargo.toml`, `crates/connectors-docs/Cargo.toml`,
  `apps/connectors/Cargo.toml`, `Cargo.lock` (cited: the consuming crates)
- `crates/connectors-build/src/gate.rs` (cited: the offline test step)
- `.github/workflows/rust-gate.yml`, `pages.yml`, `macos-host.yml`, `windows-host.yml` (cited)
- the check test (inferred: beside `crates/connectors-catalog/tests/definition_import.rs`)
- `AGENTS.md` (inferred: how to fetch the pinned release before running cargo locally)

## Acceptance

- With the committed pin, the gate check passes for all 15 selections under
  `adapters/catalog/providers/`.
- A test that adds a selection the pinned release lacks makes the check fail naming that
  connector.
- A test in which the pinned release holds a definition that does not lower makes the check fail
  naming that connector.
- `adapters/catalog`, `crates/connectors-catalog` and `apps/connectors` each list the
  test-support crate as a dev-dependency.
