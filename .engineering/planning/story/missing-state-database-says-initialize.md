---
format: aep.planning-md/3
id: story:missing-state-database-says-initialize
kind: story
status: draft
title: Every read answers metadata_missing / initialize_state for a missing state database
relations:
- decomposes: epic:review-20261123-findings
revision: 1
---
## Finding 6

6. Two reads give different recovery instructions for the same missing state database. On an initialized configuration pointed at an empty state directory, connections list returned metadata_missing / initialize_state. invoke --describe returned metadata_unavailable / retry_status. Retrying leaves the database missing. Use the initialization instruction consistently.
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/contracts/cli/v1alpha1/semantics.md#L362
https://github.com/beyond10x/connectors/blob/28c0dd7a7c07fd7ce2acc660a1095b87918c2d41/apps/connectors/src/local/invoke.rs#L1036

## Source

Source: the connectors review of 2026-11-23 (Connectors 0.74.1 with Definitions 0.8.0, checked against main 45276d755c4), handed to this session by Timo the same day. The finding is quoted from it; the links are its citations.

## Acceptance

- On an initialized configuration with an empty state directory, `connections list` and `invoke --describe` both answer `metadata_missing` with `initialize_state`; a test covers both.
