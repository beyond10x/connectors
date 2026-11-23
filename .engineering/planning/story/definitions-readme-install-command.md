---
format: aep.planning-md/3
id: story:definitions-readme-install-command
kind: story
status: draft
title: 'The Definitions README documents the working install command (consumer: beyond10x/connectors-definitions)'
relations:
- decomposes: epic:review-20261123-findings
revision: 1
---
## Finding 10

10. The Definitions 0.8.0 README calls installation unbuilt and gives --ref. Connectors 0.74.1 requires --source and --tag. That command installed the published 0.8.0 bundle in this review. Document the working command and the separate status of named-instance resolution.
https://github.com/beyond10x/connectors-definitions/blob/3c4d34154abc6fe61705aa9e2e15115238303d2d/README.md#L15
https://github.com/beyond10x/connectors-definitions/blob/3c4d34154abc6fe61705aa9e2e15115238303d2d/README.md#L68
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/contracts/cli/v1alpha1/semantics.md#L1094

## Source

Source: the connectors review of 2026-11-23 (Connectors 0.74.1 with Definitions 0.8.0, checked against main 45276d755c4), handed to this session by Timo the same day. The finding is quoted from it; the links are its citations.

## Acceptance

- The connectors-definitions README gives `--source` and `--tag`, and states that named-instance resolution is separate (story:named-instances-use-installed-definitions).
