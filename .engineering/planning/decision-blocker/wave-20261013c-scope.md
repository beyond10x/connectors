---
format: aep.planning-md/3
id: decision-blocker:wave-20261013c-scope
kind: decision-blocker
status: open
title: What wave 20261013c delivers toward running the CLI on Windows
relations:
- blocks: story:host-runs-on-windows
- serves: vision:independent-contract-adapters
revision: 1
---
## Question

What wave 20261013c delivers toward `epic:local-cli-on-macos-and-windows`: `story:host-runs-on-windows`, after the macOS release.

## Options

| option | what | cost |
|---|---|---|
| A | The whole story in one wave: D (specification, contracts, CLI test gating) first, then A (named-pipe transport and handle passing), B (capture, job objects, consumer launch) and C (files, DACLs, clocks, terminal, build tooling); a `windows-latest` job running clippy and the tests of `connectors-host` and `connectors`. | One tree, under 10G of target on Linux; local `cargo check --target x86_64-pc-windows-gnu`; Windows tests only on the runner. Adversary passes on A and B. A and B share `runtime/launch.rs`, so B waits on A. |
| B | Two waves. First D and C, with the owner, adapters and consumer launch refusing as `unsupported` on Windows, so `setup` and configuration run there; then A and B. | Two releases and two pull requests; the first release runs no operation on Windows. |

Recommended: A. The units are the same in both; B only adds a release that cannot run an operation, and A's file split already keeps the shared file to one handoff.

Default on silence: A.

Inside the story, the consumer contract gains a Windows delivery (an inherited handle named in an environment variable, in place of descriptor 3). That is a public contract addition, not a change to the Linux or macOS rule.
