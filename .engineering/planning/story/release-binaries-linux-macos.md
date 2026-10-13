---
format: aep.planning-md/3
id: story:release-binaries-linux-macos
kind: story
status: active
title: Release tags carry pre-built Linux and macOS binaries
relations:
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T10:14:13Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-13T10:14:14Z", actor: "human:timo", revision: 3}
---
## Outcome

Every release tag `v<version>` carries pre-built `connectors` binaries for Linux and macOS on its GitHub Release, with a `SHA256SUMS` file, so a user installs without a Rust toolchain. No Windows binary.

## Today

Read 2026-10-13 on wave/20261013b: no workflow attaches a binary to a release; `AGENTS.md` "Cutting a release" step 7 says the page carries the source archives GitHub generates and nothing else. `macos-host.yml` job `sign` builds a macOS binary on a `v*` tag and keeps it as a workflow artifact only, signed and notarized when the signing secrets exist.

## Scope

- `.github/workflows/release-binaries.yml` (new): on a `v*` tag push, check the tag's provenance (annotated, peels to the pushed commit, workspace version and the `CHANGELOG.md` heading agree), build `connectors` with `--locked --release` for `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`, `x86_64-apple-darwin` and `aarch64-apple-darwin` on native runners with the pinned Rust, check `connectors --version` names the tag's version, package `connectors-<version>-<target>.tar.gz` (binary, `README.md`, `CHANGELOG.md`), write `SHA256SUMS`, and upload them to the release page once it exists. macOS binaries are signed and notarized before packaging when the signing secrets exist, and stay unsigned otherwise, with a line in the run summary. The build jobs also run, without publishing, on a pull request that changes the workflow. A manual dispatch backfills an existing annotated tag.
- `.github/workflows/macos-host.yml`: its tag-only `sign` job moves into the new workflow.
- `AGENTS.md` "Cutting a release" step 7, `README.md` install section, `CHANGELOG.md`, website install guide.

## Acceptance

- A pull request run of the new workflow builds and packages all four targets, and each binary answers `--version` with the workspace version.
- On the next release tag, the Release lists four archives and `SHA256SUMS`, and `sha256sum -c SHA256SUMS` passes on the downloaded archives.
- No Windows artifact is built or published.

## Limitations

- The macOS binaries are unsigned until the signing secrets exist; an unsigned binary is quarantined by Gatekeeper when downloaded by a browser, and cannot read keychain items an earlier build wrote.
