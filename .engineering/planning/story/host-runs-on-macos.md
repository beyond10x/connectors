---
format: aep.planning-md/3
id: story:host-runs-on-macos
kind: story
status: draft
title: The connectors binary builds and runs on macOS
relations:
- decomposes: epic:local-cli-on-macos-and-windows
- serves: vision:independent-contract-adapters
- depends_on: story:platform-credential-custody
revision: 1
---
## Outcome

The `connectors` binary builds and runs on macOS: `setup`, connections, operations and the owner
process, with custody in the macOS Keychain (`story:platform-credential-custody`).

## Today

`connectors-host` calls Linux-only items with no platform gating: `SO_PEERCRED`/`ucred` peer checks
(`local/keyring.rs`, `local/runtime/channel.rs`), `memfd_create` and fd passing for consumer launch
(`local/runtime/launch.rs`, `local/runtime/artifact.rs`), pidfd process control
(`local/runtime/process.rs`, `local/owner/lifecycle.rs`, `local/owner/supervisor.rs`),
`renameat2` and `fstatfs` (`local/filesystem.rs`), `prctl` and `close_range`.
`apps/connectors/src/local.rs:337,359` reports `"os":"linux"` unconditionally.

## Design questions to settle in the specification first

- Peer credentials: `getpeereid`/`LOCAL_PEERCRED` in place of `SO_PEERCRED`.
- Executable capture without `memfd_create`, and process supervision without pidfds (kqueue
  `EVFILT_PROC`), each keeping the guarantee the Linux path gives or naming the one it cannot.
- Atomic no-replace rename: `renamex_np(RENAME_EXCL)`.
- Filesystem qualification: which macOS filesystems are admitted (APFS).

## Acceptance

- Spec first: `connectors.cli.OperatingSystem` names `macos`, and each guarantee that differs is
  modelled and validated with the newest `ess` before implementation.
- A CI job on `macos-latest` builds the workspace and runs the host and CLI suites; a test that
  needs a Linux-only fixture is gated and named in the ignored inventory.
- Linux behaviour and its tests are unchanged.
