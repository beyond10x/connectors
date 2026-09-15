---
format: aep.planning-md/1
id: story:endpoint-contract-version
kind: story
status: draft
title: Publish the endpoint-vocabulary wire contracts
relations:
- decomposes: epic:local-product
- derived_from: story:reconcile-connectors-domain-language
- supersedes: task:frozen-contract-compatibility-map
revision: 1
---
## Acceptance

A caller can perform every durable-endpoint and operation interaction speaking only the endpoint vocabulary on the wire: a published `contracts/` bundle (README, bundle.json, schema, vectors) exists for the new endpoint-management contract and for `b10x.connector-operation.v0alpha5`, the served readers accept them, the CLI uses them by default, and no serde rename mapping endpoint names onto connection wire words remains in the new contract's reader. The old published contracts still parse through their frozen readers until the retirement milestone.

## Why

The operator decided (2026-09-08 session) to stop carrying the connection wire words inside the current readers and instead create a new contract version whose wire is natively `endpoint_ref` — the "separately approved version migration" that docs/design/23-domain-language-review.md D8 reserved. Interim state: the current readers keep the old identities (`b10x.connector-connection.v0alpha1/2`) and hide the old wire words behind `#[serde(rename)]`; that layer is correct but temporary.

## Decisions this story carries

1. **Contract identity for durable-endpoint management.** The existing families are `connector-connection` (durable management, frozen at v0alpha2) and `connector-endpoint` (inventory, v0alpha1, already endpoint-vocabulary). Options:
   - (a) `b10x.connector-endpoint.v0alpha2` — one endpoint family; the management envelope becomes the family's next version and the inventory envelope folds in or stays side by side. Needs an explicit statement of what v0alpha2 contains.
   - (b) a new family such as `b10x.connector-endpoint-management.v0alpha1`, keeping inventory and management as separate surfaces.
   Neither is chosen here; the operator picks at review.
2. **Operation contract:** publish `b10x.connector-operation.v0alpha5` as a bundle — the reader already exists (`crates/protocol/src/operation/v5.rs`, endpoint_ref native, refuses connection_ref/instance_id).

## Work

- Give `crates/protocol/src/endpoint.rs` / `endpoint_v2.rs` the new contract identity, remove the serde renames, and dispatch the old identities to the restored frozen readers (`connection.rs`, `connection_v2.rs`, re-declared in lib.rs).
- Author the new bundles with vectors and manifests; extend the freeze pins to cover them once published.
- Route hosted/local servers and the client by protocol identity; CLI default moves to the new identities (operation already defaults to v5).
- State migration plan for stored `connection:` refs (for example `connection:slack:{instance_id}`), or an explicit decision to keep stored refs as opaque bytes.
- Retirement milestone for `connector-connection` v0alpha1/2 and operation ≤ v0alpha3: deprecation window, then removal of the frozen serving path. Requires its own operator approval; not part of this story's acceptance.

## Sources

- docs/design/23-domain-language-review.md D8 (legacy boundary, version migration).
- task:frozen-contract-compatibility-map (the interim serde-rename layer this story supersedes).
- crates/protocol/src/operation/v5.rs, crates/protocol/src/endpoint_inventory.rs (existing endpoint-vocabulary readers).
