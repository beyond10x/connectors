---
format: aep.planning-md/3
id: story:tool-contract-for-any-instance
kind: story
status: draft
title: A caller can retrieve tool-contract/1 for the admitted operations of any instance
relations:
- decomposes: epic:review-20261123-findings
- informed_by: epic:tool-contract-publication
revision: 1
---
## Finding 3

3. Callers cannot retrieve the standard tool contract for an arbitrary instance. Atlas assigns publication of tool-contract/1 for admitted operations to Connectors. Tavily has a generated contract. Catalog profiles and per-instance CLI and library access are still recorded as later work. Ordinary operation descriptions remain available.
https://github.com/beyond10x/atlas/blob/f924bfd92db8809a32eee112e6f8a85caf78031c/architecture/adr/0103-connectors-implements-the-tool-contract.md#L61
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/adapters/tavily/generated/tool-contract.yaml
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/.engineering/planning/epic/tool-contract-publication.md#L46

## Source

Source: the connectors review of 2026-11-23 (Connectors 0.74.1 with Definitions 0.8.0, checked against main 45276d755c4), handed to this session by Timo the same day. The finding is quoted from it; the links are its citations.

## Acceptance

- One CLI command prints tool-contract/1 for a named catalog instance's admitted operations, as Tavily's generated contract does.
