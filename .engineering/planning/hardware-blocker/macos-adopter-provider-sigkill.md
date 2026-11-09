---
format: aep.planning-md/3
id: hardware-blocker:macos-adopter-provider-sigkill
kind: hardware-blocker
status: open
title: No machine that reproduces the macOS SIGKILL of a release-built provider started through the host
relations:
- blocks: story:macos-release-provider-starts-through-the-private-copy
revision: 1
---
## What is missing

A macOS Apple-silicon machine on which a release-built provider started through the host ends on
SIGKILL before the handshake. Only the downstream adopter of D430 has one.

## Why no code change in this repository clears it

`story:macos-release-provider-starts-through-the-private-copy`, section "Status after the D430 CI
case (D440)": on `macos-latest` (macOS 26.6.2, PR 237 run 41742146148, job 126286036812) the
published 0.65.0 and a branch-built provider, copied with `cp` and with the host's private write,
start in all eight direct starts and in every start through the host; the kernel log shows no
signal and no denial. The kill does not reproduce on the CI runners this repository can use, so a
failing test cannot be written first and any change to the copy would be a guess.

## What clears it

The adopter's answers to the four questions in that story (`sw_vers`; the exit code of a plain
copy-and-run from the shell and from launchd; whether the laptop is managed; `log show --last 5m
--predicate 'eventMessage CONTAINS "connectors-image"'` right after a failed start), or access to a
machine that reproduces the kill.

## Recorded

2026-11-09, epic `epic:zero-open-issues-and-store-defects`: the story stays active and leaves every
wave of that epic's plan while this blocker is open.
