---
format: aep.planning-md/3
id: story:website-build-examples-ess-enum
kind: story
status: implemented
title: Website build refuses the CLI configuration-format enum with ESS 0.45.0
relations:
- serves: vision:independent-contract-adapters
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T10:54:35Z", actor: "human:timo", revision: 2, decided_on: {"recorded":{"test_result":2}}}
- {from: "proposed", to: "active", at: "2026-10-01T10:54:35Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"test_result":2}}}
- {from: "active", to: "implemented", at: "2026-10-01T10:54:35Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"test_result":2}}}
---
## Defect

`npm run build` in `website/` fails at its `connectors-build examples` step with the pinned ESS
0.45.0:

```
rust target cannot emit this workspace
connectors.cli.ConfigurationFormat.connectors-local/1: enum variant allocates invalid Rust identifier `ConnectorsLocal/1` in `variants:connectors.cli.ConfigurationFormat`
```

Observed 2026-09-30 on the v0.19.0 release commit `0667e89ad` and on `b50ffba79` (v0.18.0 main),
so it predates the HubSpot change. The repository gate (`connectors-build gate --msrv`) does not run
this step and is green on both.

## Acceptance

- `npm run build` in `website/` exits 0 with the pinned ESS.
- The repository gate or a CI job runs the step that failed, so a refusal like this one fails a check.

## Resolution

The example model is synthesized from only the domains the examples read (EXAMPLE_DOMAINS in crates/connectors-build/src/docs.rs, plus the example component's own domains and their references: 14 of 22), so the CLI domain and its `connectors-local/1` enum no longer reach the ESS Rust target. Wire values unchanged. The gate now synthesizes that model for rust and web, so a refusal like this fails `connectors-build gate` and CI's rust-gate.yml. Not fixed here: ESS's Rust generator ignores `code:` on enum variants (0.45.0 and ESS main); the WASM build still runs only in npm run build.
