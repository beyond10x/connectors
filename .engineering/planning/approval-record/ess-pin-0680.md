---
format: aep.planning-md/3
id: approval-record:ess-pin-0680
kind: approval-record
status: draft
title: Move the ESS pin to 0.68.0, and to 0.69.0 when released
revision: 2
---
## Stop

The directing session `b10x-chef-orchestrator` approved the ESS pin bump on 2026-10-23 under its grant: "ESS 0.68.0 pin bump approved (~/beyond10x/AGENTS.md § ESS version: \"ALWAYS upgrade to most recent ESS\"); move to 0.69.0 the same way when ess releases it."

## Scope

- `crates/connectors-spec/toolchain.json` and the seven `ess-*` git dependencies move from 0.67.0 to 0.68.0; `Cargo.lock` moves only for the 11 crates of the ESS repository.
- No breaking change applies: the 0.68.0 additions take effect from `format: ess/30`, and this specification is `ess/18`.
- The same approval covers moving to 0.69.0 when it is released.

## Evidence

`connectors-build gate --msrv` (ESS 0.68.0, aep 0.71.3) on the bumped tree: 30 steps exit 0, 3142 passed, 0 failed, 148 ignored; authored scenarios 16 passed and the same 34 known failing as under 0.67.0. A first run lost 5 of 14 `local_mcp_server` cases to a 15 s reply timeout at load average 80; they passed alone and in the second full run.
