---
format: aep.planning-md/3
id: story:host-runs-on-windows
kind: story
status: draft
title: The connectors binary builds and runs on Windows
relations:
- decomposes: epic:local-cli-on-macos-and-windows
- serves: vision:independent-contract-adapters
- depends_on: story:platform-credential-custody
- depends_on: story:host-runs-on-macos
revision: 1
---
## Outcome

The `connectors` binary builds and runs on Windows: `setup`, connections, operations and the owner
process, with custody in the Windows Credential Manager (`story:platform-credential-custody`).

## Today

Besides the Linux-only calls `story:host-runs-on-macos` lists, the host assumes Unix: the owner
transport and the v2 mutation exchange are Unix domain sockets (`local/owner/transport.rs`,
`local/runtime/writes.rs`, `local/runtime/server.rs`), consumer launch passes file descriptors,
and file admission checks the owner uid and mode bits (`local/filesystem.rs`, `local/config.rs`,
`credentials.rs`, `local/protected.rs`).

## Design questions to settle in the specification first

- Owner transport: a named pipe restricted to the current user's SID, or AF_UNIX on Windows, with
  the peer's identity checked.
- Owner-only files: an ACL that grants the current user alone, in place of mode bits.
- Process launch and supervision: job objects in place of process groups, pidfds and `prctl`.
- Terminal entry without `termios`.

## Acceptance

- Spec first: `connectors.cli.OperatingSystem` names `windows`, and each guarantee that differs is
  modelled and validated with the newest `ess` before implementation.
- A CI job on `windows-latest` builds the workspace and runs the host and CLI suites; a test that
  needs a Unix-only fixture is gated and named in the ignored inventory.
- Linux and macOS behaviour and their tests are unchanged.
