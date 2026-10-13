---
format: aep.planning-md/3
id: release-plan:connectors-v0460-platform-custody
kind: release-plan
status: active
title: 'Release 0.46.0: credential custody on macOS and Windows, ESS 0.59.0'
relations:
- serves: vision:independent-contract-adapters
- informed_by: decision-blocker:wave-20261012a-scope
revision: 2
transitions:
- {from: "draft", to: "active", at: "2026-10-13T02:49:16Z", actor: "human:timo", revision: 2}
---
## Outcome

Minor release 0.46.0: credential custody gets a backend trait in a new crate
`connectors-custody`, with a macOS Keychain and a Windows Credential Manager backend beside the
Secret Service one, chosen by build target, with no plain-text fallback; and ESS 0.59.0.

Minor rather than patch: a new crate, the `setup check` result gains `custody_backend`, and
`connectors.cli.OperatingSystem` gains `macos` and `windows`. Linux behaviour, configurations and
stores are unchanged.

## Scope

| surface | change |
|---|---|
| `ess/domains/cli.yaml`, `contracts/auth/custody/v1alpha1/semantics.md` §5.1 | `CustodyBackend`, `custody_backend`, `OperatingSystem` variants; per-platform bindings |
| `crates/connectors-custody` | trait, in-memory reference, Secret Service (moved), Keychain (`security-framework` 3.7.0), Credential Manager (`windows-sys` 0.61.2) with part entries and a header, delete rounds |
| `crates/connectors-host`, `apps/connectors` | host calls the trait; `setup check` reports the backend and the build target's `os` |
| `.github/workflows/custody-platforms.yml` | custody clippy and tests on `macos-latest` and `windows-latest`, real stores included |
| `Cargo.toml`, `crates/connectors-spec/toolchain.json` | ESS 0.57.0 to 0.59.0 |
| CHANGELOG, README, generated pages | documentation and version 0.46.0 |

Story: `story:platform-credential-custody` (https://github.com/beyond10x/connectors/issues/157);
decided in `decision-blocker:wave-20261012a-scope`, `decision-blocker:platform-custody-limits` and
`decision-blocker:macos-keychain-partition`.

## Evidence

- Wave PR https://github.com/beyond10x/connectors/pull/159: Rust gate, Custody platforms (macOS and
  Windows), Documentation validation, Planning store and Shared source gates passed on `1f3e52a68`;
  Custody platforms re-run on the same head.
- Package gates on `60eeaa592`: clippy and test of connectors-custody, connectors-host, connectors,
  connectors-docs, connectors-spec and connectors-build: 883 passed, 0 failed, 40 ignored;
  ignored inventory clean; `connectors-docs generate --check` current. Later commits touched
  connectors-custody (and docs once): its gates passed on each, 19 passed on `1f3e52a68`.
- Windows: the real Credential Manager passed the contract suite, the 64 KiB split and the largest
  material of every shipped auth profile (24,630 bytes for OAuth refresh). A delete whose calls all
  succeeded still found an entry on 3 of 5 runs; deletes now repeat in rounds.
- macOS: the real login keychain passed the contract suite and the locked-keychain refusal.
- No adversary review ran on this wave.

## Completion boundary

Not in this release: the `connectors` binary does not build for macOS or Windows
(`story:host-runs-on-macos`, `story:host-runs-on-windows`). On macOS a second binary is refused an
item an earlier one wrote (OS status -25293, verified on the runner; partition-list cause
inferred); upgrade access and release signing belong to `story:host-runs-on-macos`.
`story:slack-message-send` (channels and direct messages) follows this release.
