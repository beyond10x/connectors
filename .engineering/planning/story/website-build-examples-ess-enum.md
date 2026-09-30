---
format: aep.planning-md/3
id: story:website-build-examples-ess-enum
kind: story
status: draft
title: Website build refuses the CLI configuration-format enum with ESS 0.45.0
relations:
- serves: vision:independent-contract-adapters
revision: 1
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
