---
format: aep.planning-md/3
id: story:platform-credential-custody
kind: story
status: active
title: macOS Keychain and Windows Credential Manager custody beside the Secret Service
relations:
- decomposes: epic:local-cli-on-macos-and-windows
- serves: vision:independent-contract-adapters
- informed_by: decision-blocker:wave-20261012a-scope
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-12T20:39:18Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-12T20:39:19Z", actor: "human:timo", revision: 4}
---
## Outcome

Credential custody on macOS and Windows: a macOS Keychain backend and a Windows Credential Manager
backend beside the Secret Service one, chosen by the platform the binary was built for, with no
plain-text fallback. On a platform or machine where no qualified backend is available, custody
refuses with `custody_unavailable`, as it does on Linux today.

## Today

Custody is one concrete binding, not a trait: `Store` in
`crates/connectors-host/src/local/keyring/custody.rs:111-117` holds the Secret Service connection
and the GNOME Keyring persistence check directly. The qualification is in
`docs/local-secret-service.md`. Its semantics are owned by `contracts/auth/custody/v1alpha1/`. The
ESS model has no backend: `ess/domains/auth_bindings.yaml` says no storage backend is introduced,
`connectors.cli.LocalCustodyBinding` has only `secret_service_socket` (`ess/domains/cli.yaml:142`)
and `connectors.cli.OperatingSystem` has only `linux` (`ess/domains/cli.yaml:297`).

## Design

- Spec first: `connectors.cli.OperatingSystem` gains `macos` and `windows`; a new enum
  `connectors.cli.CustodyBackend` (`secret_service`, `macos_keychain`, `windows_credential_manager`)
  is reported by `setup check`; `LocalCustodyBinding` names the backend. The custody contract's
  guarantees (immutable scoped versions, a definite acknowledgement, `outcome_unknown` on an
  ambiguous write, no activation, unlock or prompt, no fallback) are restated per backend.
- A new crate `connectors-custody` holds the backend trait (write a new scoped version, read an
  exact version, delete, inspect state), the `Failure` vocabulary and the three backends, each under
  `#[cfg(target_os = …)]`. It has no Linux-only code outside the Linux backend, so it compiles on all
  three platforms. `connectors-host` keeps the coordinator (guards, writer lock, fences) and calls
  the trait. The Linux backend is the existing code moved behind the trait, its qualification
  unchanged.
- macOS: generic password items in the user's login keychain through `security-framework` 3.7.0
  (already in `Cargo.lock` through `rustls-platform-verifier`). Service `connectors`, account the
  scope and version ids. User interaction is disallowed, so a locked keychain answers `Unavailable`
  and never shows an unlock prompt. The data-protection keychain needs a signed binary with
  entitlements and is not used.
- Windows: `CredWriteW`, `CredReadW`, `CredDeleteW` with `CRED_TYPE_GENERIC` and
  `CRED_PERSIST_LOCAL_MACHINE` (never roaming), through `windows-sys` 0.61.2 (already in
  `Cargo.lock` through `mio`). The target name carries the scope and version ids. A credential blob
  holds at most 2,560 bytes (`CRED_MAX_CREDENTIAL_BLOB_SIZE`), against 65,536 in the model; a larger
  material is refused as `InvalidMaterial`, never split or written elsewhere. The first test
  records the largest material each shipped auth profile writes.
- No new crate is downloaded: both platform crates are already locked.

## How each backend is tested

- A contract suite generic over the trait, in `connectors-custody`, runs against an in-memory
  reference backend on every platform, and against each real backend where it runs.
- macOS Keychain: a CI job on GitHub's `macos-latest` runner runs `cargo test -p
  connectors-custody`, which exercises the real login keychain of the runner.
- Windows Credential Manager: a CI job on `windows-latest` runs the same suite against the real
  Credential Manager.
- Secret Service: the existing private dbus and gnome-keyring fixtures on Linux, unchanged.

## Acceptance

- Spec first: the enums and the binding are modelled in `ess/domains/cli.yaml` (and the custody
  contract) and validated with the newest `ess` before implementation.
- On each platform the contract suite passes against the real backend: write, read back the exact
  bytes, a second write of the same version is `Conflict`, delete, read after delete is `Missing`.
- A locked macOS keychain answers `Unavailable` without a prompt (runner test locks a test
  keychain).
- A Windows material over 2,560 bytes is refused as `InvalidMaterial` and nothing is written.
- No path stores credential material in a file or environment variable on any platform.
- Linux behaviour and its tests are unchanged.

## Not in this story

The rest of the local host is Linux-only: owner transport, consumer launch, filesystem admission and
terminal entry use Linux-only calls in 22 non-test files of `connectors-host` with no platform
gating. `story:host-runs-on-macos` and `story:host-runs-on-windows` own that. Until they land, the
`connectors` binary does not build for macOS or Windows.

## Issue

https://github.com/beyond10x/connectors/issues/157
