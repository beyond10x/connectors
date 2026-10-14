---
format: aep.planning-md/3
id: story:host-runs-on-windows
kind: story
status: implemented
title: The connectors binary builds and runs on Windows
relations:
- decomposes: epic:local-cli-on-macos-and-windows
- serves: vision:independent-contract-adapters
- depends_on: story:platform-credential-custody
- depends_on: story:host-runs-on-macos
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-13T06:25:56Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-13T06:25:57Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-13T19:52:53Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":2}}}
---
## Outcome

The `connectors` binary builds and runs on Windows: `setup`, connections, operations and the owner
process, with custody in the Windows Credential Manager (`story:platform-credential-custody`).

## Today

Read 2026-10-13 on wave/20261013b (38d36b02c), after the macOS port. Every platform branch in the host is `#[cfg(target_os = "linux")]` or `#[cfg(target_os = "macos")]`; nothing builds for Windows. 22 non-test files of `crates/connectors-host/src` use `libc`, `std::os::unix`, Unix sockets, mode bits or `termios` (by count of such lines): `runtime/launch.rs` 81, `protected.rs` 49, `owner/transport.rs` 48, `runtime/channel.rs` 47, `filesystem.rs` 44, `runtime/artifact/macos.rs` 38, `runtime/process.rs` 18, `metadata.rs` 17, `clock/timer.rs` 12, `runtime/artifact.rs` 10, `owner/lifecycle.rs` 10, and 11 files with 5 or fewer. `apps/connectors/src` has none; `crates/connectors-build/src` has 3 files. `connectors-custody` already builds and passes its contract suite on `windows-latest` (`custody-platforms.yml`).

The Unix assumptions that need a design, not a substitution:

- Descriptor passing: the CLI and the owner exchange the image and credential descriptors with `SCM_RIGHTS` (`runtime/launch.rs` `send`/`receive`). Windows has no descriptor passing over a socket.
- Consumer delivery: the consumer contract places the credential on descriptor 3 (`contracts/cli/v1alpha1/consumer-launch.md` §3, §5). Windows has no descriptor 3.
- Owner-only files: admission checks owner uid and mode bits (`filesystem.rs`, `config.rs`, `credentials.rs`, `protected.rs`).

## Design questions to settle in the specification first

Proposed mechanisms, none run yet (inferred from the Windows API, to be checked on the runner):

- Owner transport and peer: a named pipe whose ACL grants the current user's SID alone, the client's SID read from its token (`GetNamedPipeClientProcessId`, then the process token). Handles reach the peer with `DuplicateHandle` into the peer process in place of `SCM_RIGHTS`.
- Consumer delivery: the credential as one inherited, read-only handle restricted with `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`, its value given to the consumer in a named environment variable. A consumer contract change: the Windows section names it beside descriptor 3.
- Executable capture: a copy opened without `FILE_SHARE_WRITE` and held until the child is created, so no process can rewrite it between the digest check and the launch; the PE header checked in place of ELF or Mach-O.
- Process supervision and parent death: a job object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, which ends every child when the owner or the launching CLI exits, consumers included; exit through the process handle, which is not reused while held.
- Owner-only files: a DACL granting the current user alone, checked on the opened handle, in place of mode bits; no-replace rename with `MoveFileExW` without `MOVEFILE_REPLACE_EXISTING`, on NTFS.
- Clocks: `QueryUnbiasedInterruptTime` (excludes suspend) and `QueryInterruptTime` (includes it).
- Terminal entry: console mode without `ENABLE_ECHO_INPUT` in place of `termios`.

Each guarantee that differs gets a Windows row in `connectors.cli.PlatformGuaranteeStatement` and the semantics table, with the Linux and macOS values unchanged.

## Units

Disjoint files, D first:

- D, specification and CLI tests: `ess/domains/cli.yaml` (Windows rows, a weaker form where one applies), `contracts/cli/v1alpha1/{semantics,owner,private-adapter,consumer-launch}.md` (Windows sections), CLI test gating in `apps/connectors/tests/`.
- A, transport and handle passing: `owner/transport.rs`, `runtime/channel.rs`, `runtime/server.rs`, `runtime/writes.rs`, `owner/mutation.rs`, and the send/receive half of `runtime/launch.rs`.
- B, capture, launch and supervision: `runtime/artifact.rs` (+ a `windows.rs` beside `macos.rs`), `runtime/process.rs`, `owner/lifecycle.rs`, `owner/supervisor.rs`, and the consumer spawn half of `runtime/launch.rs`. A and B share `launch.rs`: A takes it first and B rebases onto A.
- C, files, clock, terminal and build tooling: `filesystem.rs`, `config.rs`, `credentials.rs`, `metadata.rs`, `protected.rs`, `approval_policy.rs`, `approval_keys.rs`, `clock/timer.rs`, `crates/connectors-build/src/`.
- The coordinator keeps the Cargo manifests (`windows-sys` 0.61.2 for the host, the version `connectors-custody` already pins, so no new crate), the `windows-latest` job in a workflow, the changelog and the generated docs.

Local check: `cargo check --target x86_64-pc-windows-gnu` with the installed rustup target and `x86_64-w64-mingw32-gcc`; tests run on the `windows-latest` runner only.

## Acceptance

- Spec first: `connectors.cli.OperatingSystem` names `windows`, and each guarantee that differs is
  modelled and validated with the newest `ess` before implementation.
- A CI job on `windows-latest` builds the workspace and runs the host and CLI suites; a test that
  needs a Unix-only fixture is gated and named in the ignored inventory.
- Linux and macOS behaviour and their tests are unchanged.

## Wave decisions

Taken inside the approved story, 2026-10-13. Each is the option the unit recommended unless noted.

Unit D (specification, d5f9bd343):

- `sealed_executable` is `weaker` on Windows (`deny_write_copy`): a process of the same user can open the copy for writing between the write handle's close and the deny-write open; NTFS refusal of that was not confirmed.
- `peer_credential_snapshot` is `weaker` (`client_token_query`); `own_image_build_measure` is `weaker` (`launch_path_read`); the other four hold.
- No separate owner-only-file guarantee row: owner-only files hold on every platform, as protected stdin does on macOS.
- The owner-only DACL check admits the current user, LocalSystem and BUILTIN\Administrators, the counterpart of what root can read on Unix; files the host creates name the current user only.
- Clocks are `QueryUnbiasedInterruptTimePrecise` and `QueryInterruptTimePrecise`: the plain forms advance about every 15.6 ms, under the clock binding's 1 ms resolution.
- Protected stdin on Windows admits an owner-only file and refuses a pipe until a producer check is specified.
- The consumer credential is one inherited read-only handle, its value in decimal in `CONNECTORS_CREDENTIAL_HANDLE` (`connectors.cli.ConsumerCredentialDelivery`).

Unit C (files, clocks, console, build tooling, 04fd3153e, 426839a0a):

- `owner_uid` on Windows is the last sub-authority of the user's SID, for the stored owner number; admission compares the full SID.
- `connectors-build` on Windows ends fixtures with `taskkill /T /F`, refuses `--tmpdir` and never counts a kubeconfig as present.
- Elevated sessions: Windows makes BUILTIN\Administrators the default owner of new files in an elevated administrator session, so SQLite's `-wal` and `-shm` files would be refused by the owner check. Taken: when the process token is elevated and holds BUILTIN\Administrators as its default owner, an owner SID of BUILTIN\Administrators is admitted as the current user's; otherwise the check is unchanged. To be implemented and tested on the `windows-latest` runner in this wave.
- Unchecked until the runner: NTFS refusing a rename of a folder with an open file below it; `MoveFileExW` against a reader without delete sharing; `AllocConsole` in a detached child.

Adversary passes on units A and B, decisions taken:

1. A consumer's job permits breakaway on request (`JOB_OBJECT_LIMIT_BREAKAWAY_OK`, never
   `SILENT_BREAKAWAY_OK`), so an owner a consumer starts leaves the job through
   `CREATE_BREAKAWAY_FROM_JOB` as `owner.md` already says. The adapter job keeps no breakaway.
   The `parent_death_kill` row and the consumer-launch Windows section say so.
2. Processes a consumer starts without breakaway end when the consumer's job closes, at the
   consumer's exit: a Windows difference from Linux, where `PR_SET_PDEATHSIG` covers the direct
   child only. Stated in the consumer-launch Windows section and the changelog's limitations.
3. Image copies: the guard that removes a copy exists before any fallible step, so every early
   return removes it; at owner start, stale `connectors-image-*` directories under `%TEMP%` whose
   copy can be deleted (no process holds it) are removed.
4. Children start with the copy's owner-only directory as current directory, never the
   launcher's. A Windows adapter or consumer is self-contained: DLLs beside the original
   executable are not found. Stated in the contracts.
5. A consumer gets `SystemRoot` as an adapter does; the contract's environment rule names it.
6. `pass_env` prefixes match names in any letter case on Windows.
7. Children are created in their job with `PROC_THREAD_ATTRIBUTE_JOB_LIST`, closing the window
   in which a suspended child is outside any job.
8. Recorded as limitations, not fixed: an inheritable handle exists for the duration of one
   creation, so a concurrent `bInheritHandles` creation elsewhere in the process could inherit
   it (no such creation in the host or CLI); `-wal`/`-shm` files left by a crashed elevated
   session are refused by a later non-elevated one until removed.

9. Windows scratch of the owner and the CLI (the sealed credential directory and the image
   copies) lives in an owner-only `run` directory under the state directory, never under
   `temp_dir()`; the owner is also started with `TMP` and `TEMP` set to that directory, so
   SQLite's temporary files do not fall back to the Windows directory. The stale-copy sweep of
   decision 3 runs over that directory.
10. Peer admission compares integrity level and restriction besides the user SID, on both
    sides: a CLI refuses an owner whose token has another integrity level or is restricted, and
    an owner answers such a CLI with a named refusal instead of dropping the stream. The
    contract's Windows peer section and the `peer_credential_snapshot` row say so.
11. The owner pipe name adds a per-state random salt kept in an owner-only file under the state
    directory, so another local user cannot predict the name and take it first.
12. `receive` refuses a frame that names one handle value twice, and closes every handle that
    arrived when it refuses a frame; the scratch guard is dropped after the directory handle; a
    request thread decrements `clients` after its pipe instance is closed.
