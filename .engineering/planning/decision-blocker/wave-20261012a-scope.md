---
format: aep.planning-md/3
id: decision-blocker:wave-20261012a-scope
kind: decision-blocker
status: open
title: Which parts of the macOS and Windows epic wave 20261012a delivers
relations:
- blocks: story:platform-credential-custody
- serves: vision:independent-contract-adapters
revision: 1
---
## Question

What wave 20261012a delivers toward `epic:local-cli-on-macos-and-windows`, released as 0.46.0.

Custody alone does not let the CLI run on macOS or Windows: 22 non-test files of
`connectors-host` use Linux-only calls (peer credentials, `memfd_create`, pidfds, `renameat2`,
`prctl`) with no platform gating, so the binary does not build there.

## Options

| option | what | cost |
|---|---|---|
| A | `story:platform-credential-custody` only: the `connectors-custody` crate with the trait and three backends, Linux moved behind it unchanged, CI jobs on `macos-latest` and `windows-latest` for that crate. The host ports follow as their own waves, macOS first. | One tree, about 8G of build on Linux. Two CI jobs per run. The CLI still does not run on macOS or Windows after 0.46.0. |
| B | A plus `story:host-runs-on-macos` in the same wave: the CLI runs on macOS after 0.46.0. | About 22 host files gain macOS branches that only the macOS runner can test; an adversary review; two or more sessions; same disk. |
| C | Custody only, host ports not planned. | As A, and the CLI never runs on macOS or Windows. |

Recommended: A. The backends are testable on their own runners now; the host ports need design
choices (process supervision, executable capture, owner transport) that are better made in their
own specification step.

Default on silence: A.
