---
format: aep.planning-md/3
id: story:macos-release-provider-starts-through-the-private-copy
kind: story
status: active
title: A release-built provider program starts on macOS through the host's private copy, again over an earlier copy
summary: A downstream adopter on a macOS Apple-silicon laptop sees every provider start end on SIGKILL before the handshake; a macOS CI case that starts a release-built provider through the host twice is red before the fix
relations:
- informed_by: story:host-runs-on-macos
- serves: vision:independent-contract-adapters
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-03T22:00:13Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-03T22:00:13Z", actor: "human:timo", revision: 3}
---
# Story: A release-built provider program starts on macOS through the host's private copy, again over an earlier copy

## Outcome

An operator on macOS whose provider programs are the published release binaries can start an instance through the
host, and start it again, instead of seeing every start end on SIGKILL before the handshake.

## Context

D430 (orchestrator, 2026-11-03). A downstream adopter on a macOS Apple-silicon laptop runs 0.65.0 from the release
archives: SHA-256 matches, no quarantine attribute, all six programs print their version when run directly, `setup
check` reports every prerequisite ready, `codesign -v` passes (ad-hoc, linker-signed). Starting a Grafana instance
through the host reports `start_failure` `exited_before_handshake`, `start_signal` 9, an empty start log, and no
crash report.

On macOS the host starts every provider from a private copy
(`crates/connectors-host/src/local/runtime/artifact/macos.rs` `capture`): a new file in a fresh `mkdtemp` directory
under `std::env::temp_dir()`, written in chunks, made mode 0500, re-read and checked against the pinned SHA-256 and
the Mach-O header, executed by path (`runtime/process.rs` `launch`) and removed right after spawn. The owner runs with
a cleared environment, so its copies go to `/tmp`. No CI case starts a release-built provider through the host on
macOS: the start tests use the debug test executable, and `.github/workflows/macos-host.yml` runs the release
programs' `--version` directly only. The cause is not established yet.

## Acceptance

- A macOS CI case on `macos-latest` builds a provider program with the release profile, starts it through the host
  (`runtime::Child`) twice, the second start after the first copy was made and removed, and both starts finish the
  handshake.
- The case is red before the fix in CI, its log shows the start's `start_failure` and signal, and it compares the
  host's copy with a plain `cp` of the same file started the same way.
- The cause, with the CI log that shows it, is sent to the orchestrator before a commit that changes the copy is
  pushed.
- After the fix the case is green on `macos-latest`, and the existing Linux and Windows start tests still pass.

## Out of Scope

Developer ID signing and notarisation of the release binaries; the CLI's refusal for a failed derivation start
(`story:connect-names-a-failed-provider-start`).

## Ambiguities

None classified. Which mechanism kills the copy (its signature validation, its location, or its removal right after
spawn) is not known yet; the CI case in Acceptance settles it from a log, so it needs no decider.

## Open Questions

None.
