---
format: aep.planning-md/3
id: story:rust-gate-on-ubuntu-2604
kind: story
status: draft
title: The Rust gate passes on Ubuntu 26.04
summary: Tests pin images they control; the gate leaves its ubuntu-24.04 pin
revision: 1
---
# The Rust gate passes on Ubuntu 26.04

## Outcome

The repository gate and the ignored CLI journeys run on `ubuntu-26.04` and pass, so the
workflow no longer needs its `ubuntu-24.04` pin.

## Why

`ubuntu-latest` moved to `ubuntu-26.04` on 2026-11-13. On that image
`consumer_launch::a_launch_the_configuration_refuses_is_refused_by_name_before_anything_starts`
was refused `invalid_configuration` (exit 2) for the `probe` consumer that pins
`/usr/bin/true`. Inferred cause: on 26.04 `/usr/bin/true` is a symbolic link (uutils
coreutils), and `Executable::source` opens the pinned path with `O_NOFOLLOW`. Cargo stops at
the first failing test binary, so other tests that pin system binaries were not reached.

## Acceptance

- A test that pins a consumer or adapter image pins a regular file it controls, not a system
  binary that may be a symbolic link.
- `rust-gate.yml` runs on `ubuntu-latest` again and the gate passes there.
