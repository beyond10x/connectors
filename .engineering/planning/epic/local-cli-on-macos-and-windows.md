---
format: aep.planning-md/3
id: epic:local-cli-on-macos-and-windows
kind: epic
status: draft
title: The local CLI runs on macOS and Windows with credentials in the platform store
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Outcome

The local `connectors` CLI runs on macOS and Windows as it does on Linux, with credentials kept only
in the platform's protected store and never in plain text.

## Why

Users on macOS and Windows cannot run Connectors today: the host is Linux-only and credentials
are kept only in the freedesktop Secret Service. Tools they use instead keep tokens in plain text.

## Stories

1. `story:platform-credential-custody`: the custody backends, testable on their platforms' CI
   runners before the host builds there.
2. `story:host-runs-on-macos`.
3. `story:host-runs-on-windows`.

Order: 1, then 2, then 3. A release follows each.
