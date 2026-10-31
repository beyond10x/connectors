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
scope:
- confidence: cited
  path: .github/workflows/macos-host.yml
- confidence: cited
  path: .github/workflows/pages.yml
- confidence: cited
  path: .github/workflows/rust-gate.yml
- confidence: cited
  path: .github/workflows/windows-host.yml
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: adapters/catalog/Cargo.toml
- confidence: cited
  path: apps/connectors/Cargo.toml
- confidence: cited
  path: crates/connectors-build/Cargo.toml
- confidence: cited
  path: crates/connectors-build/src/gate.rs
- confidence: cited
  path: crates/connectors-catalog/Cargo.toml
- confidence: cited
  path: crates/connectors-catalog/src/import.rs
- confidence: inferred
  path: crates/connectors-catalog/tests
- confidence: cited
  path: crates/connectors-docs/Cargo.toml
- confidence: inferred
  path: docs/development.md
revision: 6
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

Derived 2026-10-29 by `story-scoper` against origin/main 65c928d72. Every line is **cited** (read from the story or the tree) or **inferred** (a reading that could be wrong).

- **Primary surface:** the test-support crate that opens the pinned release and lowers one connector, plus the check that calls it first — inferred, the story leaves the crate's choice to implementation
- **Files:** `Cargo.lock` — cited, the story says it adds the matching entries
- **Files:** `adapters/catalog/Cargo.toml`, `crates/connectors-catalog/Cargo.toml`, `apps/connectors/Cargo.toml` — cited, the story names each as getting a dev-dependency
- **Files:** `crates/connectors-build/Cargo.toml`, `crates/connectors-docs/Cargo.toml` — cited, the story names them for gate code
- **Files:** `crates/connectors-build/src/gate.rs:193-194` — cited, the offline `cargo build`/`cargo test` steps that the fetch has to run before (the story's `:149-150` has drifted)
- **Files:** `.github/workflows/rust-gate.yml:56,62-63` — cited, a fetch step goes in before the gate
- **Files:** `.github/workflows/pages.yml:48-52`, `.github/workflows/macos-host.yml:43`, `.github/workflows/windows-host.yml:51` — cited, the story says each fetches the release first
- **Files:** a committed pin lock at the repository root — inferred, the story says the path is chosen at implementation
- **Files:** the check test, beside `crates/connectors-catalog/tests/definition_import.rs` — inferred
- **Symbols:** `definition_name` at `crates/connectors-catalog/src/import.rs:561-568`, `shipped_selections` at `:580` — cited (the story's `:542-549` and `:561-575` have drifted)
- **Also likely:** a new crate under `crates/`, which the `crates/*` members glob in `Cargo.toml:4` picks up without a root manifest edit — inferred
- **Documents:** `docs/development.md:49-55`, which describes the gate and `rust-gate.yml` (`AGENTS.md:134` points there for gate commands), gets the local fetch-before-cargo step — inferred
- **Confidence:** medium — the story names every manifest and workflow, but the crate holding the support function, the lock path and where the fetch runs are left to implementation
- **Would collide with:** any unit touching `Cargo.lock`, the five crate manifests above, `crates/connectors-build/src/gate.rs`, the four workflows above, or `docs/development.md`
- **Stale in this body:** the tree has 16 provider directories and 17 selection files on 65c928d72 (`chatgpt` and `claude-subscription` arrived after 9201b1177), not 14 and 15; whether any connectors-definitions tag holds those two is not established
- **Safety fact:** the pinned release enters only tests and the gate, never a shipped binary's build: the new edges are dev-dependencies on the catalog provider and `apps/connectors`, and only `connectors-build`/`connectors-docs` (repo tools) get normal ones — level 2 (`.github/workflows/release-binaries.yml:124-125` builds only `$BINARIES`), unproven; under option C this was false by design

## Acceptance

- With the committed pin, the gate check passes for all 15 selections under
  `adapters/catalog/providers/`.
- A test that adds a selection the pinned release lacks makes the check fail naming that
  connector.
- A test in which the pinned release holds a definition that does not lower makes the check fail
  naming that connector.
- `adapters/catalog`, `crates/connectors-catalog` and `apps/connectors` each list the
  test-support crate as a dev-dependency.
