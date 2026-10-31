---
format: aep.planning-md/3
id: story:generated-output-regenerates-in-a-fresh-checkout
kind: story
status: draft
title: ESS-generated outputs regenerate in a fresh checkout with the command the gate names
tags:
- d298
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Problem

In a fresh managed worktree (wave D298, 2026-10-31), `cargo run --locked -p connectors-build --
service-metadata` refused to regenerate `crates/connectors-core/generated/operation-types`:

```text
error: unowned output destination: Cargo.toml; adopt exact generated reference bytes explicitly
```

`mcp-bindings` and `cli` refused the same way. The ownership record ESS writes beside generated
output (`.ess-output/`) is not committed, so a new checkout has none, and the documented regenerate
command cannot run there. The gate's `--check` mode compares without it and does report the drift
("public operation metadata types drift; run service-metadata"), so the gate names a remedy that
fails in the checkout that needs it. Wave D298 regenerated with `ess generate types` directly and
copied the output, because only the recorded digests differed.

## Outcome

Regenerating every ESS-generated output with the command the gate names works in a fresh checkout
without a manual adoption step, or the gate's message names the adoption step that does.

## Acceptance

- In a fresh worktree, after an ESS source change, `connectors-build service-metadata`, `cli` and
  `mcp-bindings` regenerate their outputs, or their refusal names the exact adoption command.
- `docs/development.md` describes the procedure that works.
