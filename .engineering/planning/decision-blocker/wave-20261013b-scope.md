---
format: aep.planning-md/3
id: decision-blocker:wave-20261013b-scope
kind: decision-blocker
status: cleared
title: What wave 20261013b delivers toward running the CLI on macOS
relations:
- blocks: story:host-runs-on-macos
- serves: vision:independent-contract-adapters
revision: 2
transitions:
- {from: "open", to: "cleared", at: "2026-10-13T04:58:49Z", actor: "human:timo", revision: 2}
---
## Question

What wave 20261013b delivers toward `epic:local-cli-on-macos-and-windows`, released after the Slack send.

## Options

| option | what | cost |
|---|---|---|
| A | `story:host-runs-on-macos` in four units: the platform guarantees in the specification and CLI test gating (D) first, then peer credentials and sockets (A), capture and process control (B), and filesystem, clock and build tooling (C) in parallel. Plus a `macos-latest` job running clippy and the tests of `connectors-host` and `connectors`, and a signing job that skips while its secrets are absent. | One tree, under 10G of target on Linux; local `cargo check --target aarch64-apple-darwin`; macOS tests only on the runner. An adversary review on B. |
| B | A without signing: the signing job comes when the account exists. | Saves one workflow job now. Upgrade access stays open either way. |

Recommended: A. The skipping job states what is missing on every run, and the secret names are fixed already.

Default on silence: A.

## Decided

Option A, 2026-10-13: wave 20261013b delivers story:host-runs-on-macos in units D, A, B and C, a macos-latest CI job for connectors-host and connectors, and a release signing job that skips with a stated line until the signing secrets exist. Guarantees macOS cannot give the same way are named in the specification, never silently dropped. One pull request; released after the Slack send release.
