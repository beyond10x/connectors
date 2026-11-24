---
format: aep.planning-md/3
id: story:one-pinned-rust-toolchain-for-the-local-gate-and-the-forge
kind: story
status: active
title: One pinned Rust toolchain for the local gate, the build server and the forge
relations:
- serves: vision:independent-contract-adapters
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-11-24T05:19:07Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-11-24T06:30:10Z", actor: "human:timo", revision: 7}
---
## Defect

The repository has no `rust-toolchain.toml`. Three compilers build it:

- the forge: 11 steps in 7 workflow files run `rustup toolchain install 1.98.1` and `rustup
  default 1.98.1` (`.github/workflows/rust-gate.yml:49-50`, `:141-142`,
  `windows-host.yml:44-45`, `macos-host.yml:44-45`, `:99-100`, `:217-218`,
  `custody-platforms.yml:48-49`, `pages.yml:50-51`, `slow.yml:42-43`,
  `release-binaries.yml:67-68`, `:123-124`), and each step's name holds the version too. The
  steps differ in what else they install: components (`rustfmt`, `clippy`) and targets
  (`"$TARGET"` in the release and macOS build jobs, `wasm32-unknown-unknown` in `pages.yml`);
- the workstation: rustup's `stable`, rustc 1.99.1 on 2026-11-24;
- the build server the pool sends jobs to: rustup's `stable`, 1.100.0 until 2026-11-24
  05:00:51 CET, 1.99.1 since.

The folder rule "The local gate equals the forge" asks for one pinned rustc that the local gate
and CI both read. On 2026-11-24 the host operator declared every result of a build-server run
before 05:00:51 CET void for a tree without a pin; this repository's trees are such trees. A
lint or a test that differs between 1.98.1 and the local compiler shows on the forge only.

## Change

1. `rust-toolchain.toml` at the repository root: `channel = "1.98.1"`, `profile = "minimal"`,
   `components = ["rustfmt", "clippy"]`. It lists no target.
2. Each of the 11 workflow steps that install Rust installs the toolchain the file names and
   then prints `rustc --version`. No workflow holds a Rust version or a channel name
   (`stable`, `beta`, `nightly`) as text, in a command or in a step name. A step that needs a
   target adds it to the pinned toolchain (`rustup target add`).
3. `release-binaries.yml` can be started by hand for an existing tag and then checks out that
   tag's commit (`:23-31`, `:64`). A tag from before this story has no `rust-toolchain.toml`.
   The two install steps of that workflow end non-zero for such a checkout, with a message
   that names the missing file and the tag, before any build. They do not fall back to a
   version.
4. The text that states the version outside the workflows names the file instead:
   `docs/development.md:10` and `:85`. `docs/verification.md:45` records a past run and stays.
   `README.md:299` and `website/README.md:12` state the minimum (1.91) and stay.
5. A test in `crates/connectors-build/tests/main.rs` (a module of the crate's one test binary)
   reads every file under `.github/workflows` and fails when a step runs `rustup toolchain
   install` or `rustup default` with a version or a channel name, when a step name holds a
   Rust version, or when a job runs `cargo` with no step before it that installs from the
   file.
6. `--msrv` is unchanged: the gate calls `cargo +1.88.0` and `cargo +1.91.0`
   (`crates/connectors-build/src/gate.rs:37-39`, `:331`, `:353`, `:367`), and `+<toolchain>`
   outranks the file.

## What the pin also changes

- Every cargo or rustc call below the root reads the file: the gate's temporary root
  (`<root>/.local/tmp/gate-*`, `gate.rs:127-130`), the tests that run `cargo
  generate-lockfile` and `rustc --test` there, `cargo install` in `Taskfile.yml`, and the
  nested manifests (`website/examples/realization`, `apps/connectors-cli-contract`,
  `adapters/mcp/generated/*`, `crates/connectors-core/generated/operation-types`). They move
  from `stable` to 1.98.1, the forge's compiler.
- The documentation build compiles `website/examples/realization` for
  `wasm32-unknown-unknown`, so the pinned toolchain needs that target on the workstation and
  on the build server.
- Release binaries: the forge builds them with 1.98.1 today, so they do not change.

## Acceptance

1. In the repository root on the workstation, `rustc --version` prints `rustc 1.98.1
   (48a229cea 2026-09-01)`, `cargo clippy --version` prints `clippy 0.1.98` and `cargo fmt
   --version` prints `rustfmt 1.9.0-stable`. Before the change the first prints 1.99.1.
2. A pool job on the build server prints the same compiler: `slot rustc --version` run from
   the managed worktree (the pool runs the job in its copy of that tree on the server, which
   holds the file) prints `rustc 1.98.1 (48a229cea 2026-09-01)`.
3. `rg -n 'rustup (toolchain install|default) [0-9a-z]' .github/workflows` finds no line, and
   `rg -n '1\.98\.1' .github/workflows docs/development.md` finds no line.
4. In the forge run of the pull request, each install step that ran prints `rustc 1.98.1
   (48a229cea 2026-09-01)`. All 11 steps run there: the two of `release-binaries.yml` run
   because the pull request changes that file (`pull_request` with `paths`, `:35-38`) and
   publish nothing.
5. `rg -n 'rust-toolchain\.toml' docs/development.md` finds two lines, where lines 10 and 85
   named the version before.
6. The test of change 5, named `no_workflow_names_a_rust_version`, passes on the changed
   workflows and fails on three copies: one with `rustup default 1.98.1`, one with `rustup
   default stable`, one whose job runs `cargo` with no install step. `cargo nextest run
   --locked -p connectors-build -E 'test(no_workflow_names_a_rust_version)'` prints its time,
   under 5 s.
7. A test named `a_release_build_of_a_tag_without_the_pin_is_refused` takes the script of each
   of the two install steps of `release-binaries.yml` and runs it with `sh` in an empty
   directory, with the tag in the environment variable the step reads (`TAG`, set from the
   workflow's tag input) and with `rustup` replaced by a stub that records its calls. The
   script ends non-zero, its output holds `rust-toolchain.toml` and the tag, and the stub
   recorded no call. The test's time is named, under 5 s.
8. The pull request's checks pass: "repository gate", "ignored CLI journeys", "host and CLI
   on windows-latest", "host and CLI on macos-latest", "provider programs on macos-latest",
   "release-built providers start through the host on macos-latest", "custody on
   windows-latest", "custody on macos-latest", "Build documentation", "slow".
9. `SLOT_TIMEOUT=30m cargo run --locked -p connectors-build -- examples` (the step of
   `pages.yml:75`, which builds the `wasm32-unknown-unknown` example) ends 0 in a pool job on
   the build server.
10. In the repository root with the file present, `cargo +1.91.0 --version` prints `cargo
    1.91.0`, and the existing test that asserts the gate's `+1.88.0` argument
    (`gate.rs:1071-1073`) passes.

## Decided

- The pin is 1.98.1. `crates/connectors-tool-contracts/Cargo.toml:7` sets `rust-version =
  "1.98"`, the forge builds every release with 1.98.1, and 1.99.1 would change the released
  binaries and the formatter (rustfmt 1.9.0 in 1.98.1, 1.10.0 in 1.99.1). A move to a newer
  compiler is a change of one line in the file and its own pull request.

## Not in this story

- Rust 1.88.0 is not installed on the workstation (`rustup toolchain list`, 2026-11-24), and
  no workflow runs `gate --msrv`. That gap exists before the pin and stays.

## Shares files with

- `story:the-local-gate-lints-the-windows-target` (`rust-gate.yml`, `docs/development.md`):
  built after this story.
- `story:definitions-pin-covers-catalog` (draft; four workflows, `gate.rs`,
  `docs/development.md`), `story:windows-host-job-releases-the-cli-binary` (active;
  `windows-host.yml`, `macos-host.yml`), `story:gate-runs-a-dependency-advisory-check`
  (`rust-gate.yml`), `story:ci-planning-only-pull-requests-skip-tests` and
  `story:rust-gate-on-ubuntu-2604` (workflow files). This story changes only the install
  steps; it is not built in one wave with any of them.

## Answered

- The build server holds 1.98.1 already (the pool's owner, 2026-11-24 about 05:06 CET: the
  same toolchain list as the workstation, no rustup download on the server), and the pool gets
  a check that holds a job whose toolchain differs from the workstation's.
- Probe on the workstation, rustup 1.29.1: in a directory holding the file with `channel =
  "1.98.1"` and the two components, `rustc --version` prints `rustc 1.98.1 (48a229cea
  2026-09-01)` and `cargo clippy --version` prints `clippy 0.1.98`.
- Not checked yet: whether the build server's 1.98.1 holds `rustfmt`, `clippy` and the
  `wasm32-unknown-unknown` target (acceptance 2 and 9 show it).
- Not checked yet: the rustup version of each hosted runner image and of the Arch container
  of the journeys job. An install that reads the file needs rustup 1.28 or later; acceptance
  4 shows it per step.
