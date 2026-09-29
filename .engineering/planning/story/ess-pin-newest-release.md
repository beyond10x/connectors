---
format: aep.planning-md/3
id: story:ess-pin-newest-release
kind: story
status: draft
title: Move the ESS pin to the newest release
relations:
- serves: vision:independent-contract-adapters
revision: 1
---
## Problem

The repository pins ESS 0.40.0 (`crates/connectors-spec/toolchain.json`), and `ess` on PATH is 0.42.0 since 2026-09-29, observed by the owner-build-handshake implementor. The gate refuses a mismatched version, so a machine that stays current cannot run the gate without `CONNECTORS_ESS` pointing at an older release.

## Acceptance

- The ESS pin and the `ess-*` crates move to the newest ESS release; generated outputs are regenerated; the full gate passes with the plugin-installed `ess` on PATH.
