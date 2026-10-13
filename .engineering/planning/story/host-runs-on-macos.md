---
format: aep.planning-md/3
id: story:host-runs-on-macos
kind: story
status: implemented
title: The connectors binary builds and runs on macOS
relations:
- decomposes: epic:local-cli-on-macos-and-windows
- serves: vision:independent-contract-adapters
- depends_on: story:platform-credential-custody
- informed_by: decision-blocker:macos-keychain-partition
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T04:58:49Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-13T04:58:49Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-10-13T11:49:13Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Outcome

The `connectors` binary builds and runs on macOS: `setup`, connections, operations and the owner
process, with custody in the macOS Keychain (`story:platform-credential-custody`).

## Today

Read 2026-10-13 from the tree at 4b938b086. `apps/connectors/src/local.rs:337,359` already reports `std::env::consts::OS`, and `connectors.cli.OperatingSystem` already names `macos` (`ess/domains/cli.yaml:299-301`). Linux-only calls in `crates/connectors-host/src/local/` with no platform gating:

- Peer credentials: `SO_PEERCRED` in `runtime/channel.rs:34-58`.
- Executable and credential capture: sealed `memfd_create` in `runtime/artifact.rs:22-24,74-76` (with an ELF-only header check at `:71`) and `runtime/launch.rs:39-62`; exec through `/proc/self/fd/N` at `launch.rs:266`, `runtime/process.rs:42`, `owner/transport.rs:694,783`; `MSG_NOSIGNAL` and `MSG_CMSG_CLOEXEC` at `launch.rs:118,156`.
- Process control: pidfds in `runtime/process.rs:69,497-503`, `owner/lifecycle.rs:66-87`, `owner/supervisor.rs:466,478`; `PR_SET_PDEATHSIG` at `launch.rs:283` and `process.rs:54`; `close_range` at `launch.rs:317`.
- Filesystem: `renameat2(RENAME_NOREPLACE)` at `filesystem.rs:226,255`.
- Clock: `CLOCK_BOOTTIME` at `clock/timer.rs:30,44,46`.
- Own build: `/proc/self/exe` at `owner/transport.rs:37`.
- Protected stdin: `/proc/self/fd/0`, `pipe:[` and `/proc/<parent>` at `protected.rs:168-186`.
- Outside the host: `crates/connectors-build/src/ignored.rs:538` calls `prctl(PR_SET_CHILD_SUBREAPER)` unguarded.

No new crate: libc 0.2.190 (`Cargo.lock`) has every Apple call named below.

## Design questions to settle in the specification first

- Peer credentials: `getpeereid`/`LOCAL_PEERCRED` in place of `SO_PEERCRED`.
- Executable capture without `memfd_create`, and process supervision without pidfds (kqueue
  `EVFILT_PROC`), each keeping the guarantee the Linux path gives or naming the one it cannot.
- Atomic no-replace rename: `renamex_np(RENAME_EXCL)`.
- Filesystem qualification: which macOS filesystems are admitted (APFS).

- Guarantees macOS cannot give the same way (scoped 2026-10-13, inferred until run): a sealed executable (a 0500 copy in a 0700 directory can be rewritten by the same user before exec), parent-death kill (no `PR_SET_PDEATHSIG`; an adapter can watch its fd-3 channel, a consumer cannot), atomic close-on-exec on received descriptors, exact-process signalling unless reaping and signalling share one lock, and measuring the running image after the file is replaced. Each is named in the specification as a platform guarantee, with the Linux value unchanged.

## Units

Disjoint files, D first:

- D, specification and CLI tests: `ess/domains/cli.yaml` (a platform guarantee enum), `contracts/cli/v1alpha1/{private-adapter,consumer-launch,owner,semantics}.md`, `apps/connectors/tests/{support/socket.rs,consumer_launch.rs,failed_connect.rs,failed_connect_adversary.rs}`.
- A, peer credentials, sockets and stdin: `runtime/channel.rs`, `runtime/launch.rs`, `protected.rs`, and their tests.
- B, capture and process control: `runtime/artifact.rs`, `runtime/process.rs`, `owner/lifecycle.rs`, `owner/supervisor.rs`, `owner/transport.rs`.
- C, filesystem, clock and build tooling: `filesystem.rs`, `clock/timer.rs`, `crates/connectors-build/src/ignored.rs`.
- The coordinator keeps the Cargo manifests, the workflows (a `macos-latest` job running clippy and the tests of `connectors-host` and `connectors`), the changelog and the generated docs.

## Release signing

The signing job reads `MACOS_SIGNING_CERT_P12_BASE64`, `MACOS_SIGNING_CERT_PASSWORD` and `APPLE_TEAM_ID`; notarization reads `APPLE_API_KEY_P8`, `APPLE_API_ISSUER_ID` and `APPLE_API_KEY_ID`. Until they are set on the repository, the job skips with a line saying so, the upgrade-access check stays recorded and not failing, and the release notes tell macOS users that an upgraded binary re-stores its credentials.

## Acceptance

- Spec first: `connectors.cli.OperatingSystem` names `macos`, and each guarantee that differs is
  modelled and validated with the newest `ess` before implementation.
- A CI job on `macos-latest` builds the workspace and runs the host and CLI suites; a test that
  needs a Linux-only fixture is gated and named in the ignored inventory.
- Linux behaviour and its tests are unchanged.

## Upgrade access

Verified: on the `macos-latest` runner (https://github.com/beyond10x/connectors/pull/159, run 38322932181) a second binary reading a login keychain item that an earlier binary created, with a decrypt ACL that admits any application, gets `Unavailable`, OS status -25293 (`errSecAuthFailed`). The custody step records this without failing.

Inferred, not observed: the cause is the item's partition list, checked beside the ACL since macOS 10.12. It names the creating binary, and changing it needs the keychain password. An upgraded `connectors` would therefore not read credentials an earlier build stored.

This story makes an upgraded binary read what an earlier one wrote, and the second-binary step becomes a failing check again.

Open item: signing the release binary, so that the item's partition is the team id and survives upgrades, needs an Apple developer account. That is decided when this story starts.

## Decisions taken in the wave

Taken 2026-10-13 inside this story's approved scope:

- (a) Consumer execution: the contract follows the code. `consumer-launch.md` § 7 said `posix_spawn` with `POSIX_SPAWN_CLOEXEC_DEFAULT`; the CLI forks and execs the image copy by path and marks every descriptor from 4 up to the descriptor limit close-on-exec before exec, because libc 0.2.190 has no `posix_spawn_file_actions_addinherit_np` to keep descriptor 3. The section now says so.
- (b) Peer pid: the adapter entry (`runtime/server.rs`) requires the peer pid to equal its parent's process id on both platforms, which is stricter than "used only for its sign". The owner and the supervisor use it only for its sign. The semantics table row and `private-adapter.md` now say both.
- (c) Protected stdin on macOS: the same rule through other mechanisms (a FIFO with no `F_GETPATH` path; the parent's owner through `proc_pidinfo`, same session, not pid 1). A paragraph in the semantics macOS section; no guarantee row changes.
- (d) `setup check` reporting the weakened guarantees is not in this wave: `story:setup-check-reports-weakened-guarantees` (draft).

Gated to Linux, because they need the Secret Service fixture or a socket path past the macOS `sun_path`: the custody and approval-key test modules of `connectors-host`, `production_cli_approval_issuance_and_restart`, and `apps/connectors/tests/socket_path_adversary.rs`.
