---
format: aep.planning-md/3
id: story:definitions-install-requires-a-valid-configuration
kind: story
status: draft
title: definitions install starts only after a successful configuration read
relations:
- decomposes: epic:review-20261123-findings
revision: 1
---
## Finding 9

9. The definitions installer does not apply all configuration validation before proceeding. Source inspection shows that some parser errors fall through and the installer starts without a successful configuration read. The published contract requires that read. Have the installer use the same successful validation result as other configuration-dependent commands.
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/contracts/cli/v1alpha1/semantics.md#L1112
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/apps/connectors/src/local.rs#L649
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/apps/connectors/src/local/definitions_install.rs#L14

## Source

Source: the connectors review of 2026-11-23 (Connectors 0.74.1 with Definitions 0.8.0, checked against main 45276d755c4), handed to this session by Timo the same day. The finding is quoted from it; the links are its citations.

## Acceptance

- A configuration with a parser error makes `definitions install` refuse before any download, with the same refusal other configuration-dependent commands give.
