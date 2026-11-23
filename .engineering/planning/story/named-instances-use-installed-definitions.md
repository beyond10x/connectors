---
format: aep.planning-md/3
id: story:named-instances-use-installed-definitions
kind: story
status: draft
title: Named instances resolve through the installed definitions release, keeping Slack file.upload and Jira's field clear
relations:
- decomposes: epic:review-20261123-findings
- informed_by: epic:catalog-connectors-only-in-definitions
revision: 1
---
## Finding 1

1. Installing a definitions release does not change which catalog named instances use. Installing 0.8.0 succeeded and cached 151 files. GitHub's repo.get could then be described by file. The named catalog still showed the same 15 built-in declarations, without GitHub. The CLI contract documents this interim behavior. The accepted design calls for named instances to use the installed release.
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/contracts/cli/v1alpha1/semantics.md#L1112
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/apps/connectors/src/local/declarations.rs#L9
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/.engineering/planning/story/instances-resolve-through-locked-index.md#L20
Two of the 16 shared definition files already differ. The built-in Slack bot selection declares file.upload. Describing that operation through the released Slack file returned not_found. The built-in Jira connector permits a field clear as the only edit, while the external definition omits that rule. Preserve both behaviors when changing the catalog source.
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/adapters/catalog/providers/slack/operations.json#L124
https://github.com/beyond10x/connectors-definitions/blob/3c4d34154abc6fe61705aa9e2e15115238303d2d/connectors/slack.yaml#L134
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/adapters/catalog/providers/jira/operations.json#L75
https://github.com/beyond10x/connectors-definitions/blob/3c4d34154abc6fe61705aa9e2e15115238303d2d/connectors/jira.yaml#L99

## Source

Source: the connectors review of 2026-11-23 (Connectors 0.74.1 with Definitions 0.8.0, checked against main 45276d755c4), handed to this session by Timo the same day. The finding is quoted from it; the links are its citations.

## Acceptance

- After `definitions install` of a release, `catalog` lists the release's declarations (GitHub included) for named instances.
- Slack `file.upload` describes through the definitions path, and Jira's clear-only edit rule holds there; a test covers both.
