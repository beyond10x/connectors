---
format: aep.planning-md/1
id: story:ess-executable-pin
kind: story
status: draft
title: Resolve the pinned ESS executable from a repository record
summary: Gate and generation tests pick the ESS binary matching a repository pin file instead of whatever ess is first on PATH.
relations:
- derived_from: specification:contract-driven-connectors-design
- informed_by: story:full-review-remediation
- informed_by: story:gitlab-spec-service
revision: 1
---
## Context

The GitLab generation path pins ESS 0.9.2 (`crates/connectors-spec/src/v2.rs:8`, `check_ess` at `:562-570`). Which `ess` binary runs is decided outside the repository: `crates/connectors-build/src/main.rs` defaults `--ess` to the bare name `ess`, and `crates/connectors-spec/tests/generation.rs:6` reads `CONNECTORS_ESS` and otherwise falls back to PATH.

Observed on 2026-09-08 on the operator's machine: `~/.cargo/bin/ess` reports `ess 0.18.0`, `~/.local/bin/ess` reports `ess 0.9.2`. The remediation shell resolved 0.9.2 (`docs/evidence/review-2026-09-08/toolchain.txt`) and the gate passed; the review shell resolved 0.18.0 and 2 of 5 `generation` tests failed with `generation requires ess 0.9.2; found ess 0.18.0`, and `target/debug/connectors-build gate` refused before running any check. The README gate command (`README.md:27`) passes no `--ess`; the workaround lives only in `docs/gitlab-generation.md:12-18`.

The pass/fail of the gate therefore depends on the caller's PATH order, and no file in the repository records which executable is the correct one.

## Specification

`contracts/service/v1alpha1/semantics.md` "Specification and realization" and `spec-kinds/adapter/v2/semantics.md` govern generation; `docs/design.md:1164` already requires pinning and checking actual tool versions. No new domain entity is introduced; `ess/domains/declarations.yaml` is unchanged.

## Acceptance

With both a 0.18.0 and a 0.9.2 `ess` on PATH and 0.18.0 first, `cargo run --locked -p connectors-build -- gate --msrv` as written in `README.md` passes without `--ess` or `CONNECTORS_ESS`, and `cargo test -p connectors-spec --test generation` passes 5/5. With no 0.9.2 binary on PATH, both refuse with a message that names the repository pin record and the searched locations. `--ess` and `CONNECTORS_ESS` still override. The pinned version has exactly one editable owner in the repository, and the generated manifest's recorded `ess` value is checked against it.

## Scope

- New: one pin record, proposed `crates/connectors-spec/toolchain.json` (`{"ess":"0.9.2"}`), read with `include_str!` so `ESS_VERSION` in `v2.rs:8` is derived from it, not duplicated. Not under `ess/`, so `ess validate --path ess` does not read it.
- `crates/connectors-spec/src/v2.rs`: resolver that, given a bare name, walks PATH entries in order and returns the first executable whose `--version` matches the pin; explicit paths are checked as today.
- `crates/connectors-build/src/main.rs`, `src/gate.rs`: use the resolver for the `--ess` default; propagate the resolved absolute path as `CONNECTORS_ESS`.
- `crates/connectors-spec/tests/generation.rs`: use the resolver when `CONNECTORS_ESS` is unset.
- `README.md`, `docs/gitlab-generation.md`: state the pin file and the search rule; drop the PATH-order caveat.
- Not in scope: changing the pinned version (see `story:ess-pin-upgrade`).

## Verification

Run the README gate command twice, once with `~/.cargo/bin` first on PATH and once with `~/.local/bin` first; both must print `gate: all checks passed`. Run the generation tests with `CONNECTORS_ESS` unset in the 0.18.0-first shell. Temporarily hide the 0.9.2 binary and confirm the refusal text names the pin record. Existing 35 tests remain green; record the commands and output in `docs/verification.md`.

## Progress

Drafted 2026-09-08 from the full-review re-check. Not started.
