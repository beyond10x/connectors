---
format: aep.planning-md/3
id: release-plan:connectors-v0480-macos-host
kind: release-plan
status: active
title: 'Release 0.48.0: the CLI on macOS, and binaries on every release'
relations:
- serves: vision:independent-contract-adapters
- informed_by: decision-blocker:wave-20261013b-scope
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-13T11:49:04Z", actor: "human:timo", revision: 2}
---
## Outcome

Minor release 0.48.0: the CLI and its local host build and run on macOS (tested on Apple silicon; the x86_64 build is built, not tested), and every release tag from this one on carries pre-built `connectors` binaries for Linux and macOS (x86_64 and aarch64) with a `SHA256SUMS` file.

Minor rather than patch: a new platform, seven platform guarantees in the CLI semantics, and binary release assets.

## Scope

| surface | change |
|---|---|
| `crates/connectors-host` | macOS peer credentials, owner socket, capture and launch from a private copy, kqueue child watching, `renamex_np`, suspend-aware clocks, protected stdin; owner start-up and greeting on macOS |
| `ess/domains/cli.yaml`, `contracts/cli/v1alpha1/*` | `connectors.cli.PlatformGuarantee`, `PlatformGuaranteeStatement`, macOS sections |
| `apps/connectors/tests`, host tests | Linux-only fixtures gated; macOS cases |
| `.github/workflows/macos-host.yml` | host and CLI suites on `macos-latest` for every pull request and push to `main` |
| `.github/workflows/release-binaries.yml` | on a `v*` tag: provenance, four native builds, macOS signing when the secrets exist, archives and `SHA256SUMS` uploaded to the release page |
| docs, website, README, AGENTS.md, CHANGELOG | macOS support, install from a release binary, release step 7 |

Stories: `story:host-runs-on-macos`, `story:release-binaries-linux-macos`; decided in `decision-blocker:wave-20261013b-scope`.

## Evidence

- Wave PR https://github.com/beyond10x/connectors/pull/163, merged as `13caff97e`: Rust gate (run 38386593221), macOS host (run 38386593205, host and CLI suites on macos-latest), Custody platforms, Documentation validation, Planning store and Shared source gates passed on `f326e69a5`, after two macOS rounds that ended at the job timeout (runs 38358682116, 38370520098) and one red (38380511562).
- Package gates on `f326e69a5`: clippy and test of connectors-host (lib 368 passed, 0 failed, 33 ignored) and connectors; macOS clippy for aarch64-apple-darwin exit 0.
- Unit B adversary cases (6c27862dd) ran on macOS for the first time in run 38386593205 and passed after the capture-drop fix (c81ccd45e).
- The release-binaries workflow builds all four targets on the release pull request; the assets themselves are verified on the tag.

## Completion boundary

- macOS binaries are unsigned: signing and notarization need the operator's Apple developer account; an unsigned binary is quarantined on a browser download and cannot read keychain items an earlier build wrote.
- On macOS a consumer outlives a launcher that dies first (no parent-death signal), and a process of the same user can rewrite the private copy between its digest check and exec.
- Windows does not run the CLI yet: `story:host-runs-on-windows` is the next wave.
