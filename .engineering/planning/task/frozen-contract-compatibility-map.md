---
format: aep.planning-md/1
id: task:frozen-contract-compatibility-map
kind: task
status: draft
title: Restore frozen contract readers behind an explicit compatibility map
relations:
- decomposes: story:reconcile-connectors-domain-language
revision: 1
---
## Acceptance

The protocol bundles test target passes: `cargo test -p protocol --test bundles` exits 0 with `authentication_predecessor_artifacts_and_readers_stay_frozen` green against the original sha pins, and every v0alpha1/v0alpha2 connection and operation vector agrees with its strict reader.

## The problem

The interrupted naming migration renamed wire-visible identifiers inside the sha-pinned predecessor readers, which changes published contract bytes:

- `crates/protocol/src/connection.rs` was renamed to `endpoint.rs` with `connection_ref` → `endpoint_ref` and `source_connection_ref` → `source_endpoint_ref` field renames (250 diff lines against the frozen bytes).
- `crates/protocol/src/operation/legacy.rs` renamed `ConnectionSummary`/`connection_ref`/`connections` (36 diff lines); `operation/schema.rs` and `operation/wire.rs` follow (14 and 8 lines).
- The contracts shipped in v0.7.0 through v0.7.2 (`git tag --contains 564dfb90`), so their wire bytes are frozen. The bundles test target fails 19 tests on this at the working-tree state committed with this task.

Two wire-restoring `#[serde(rename = "source_connection_ref")]` attributes were already added on `endpoint.rs`; they are interim and are superseded by this task's repair.

## The repair, per the approved design

docs/design/23-domain-language-review.md D8: published bytes stay behind their named legacy boundary until a separately approved version migration. Concretely:

1. Restore the four pinned sources byte-exact from the frozen shas.
2. Reintroduce the endpoint vocabulary additively: re-export aliases for type names; consumer sites that touch legacy wire structs use the frozen field spellings, current interfaces (operation v5, endpoint_inventory) keep the new ones.
3. Write the explicit compatibility map D8 requires: per contract version, which wire words are frozen and which Rust names moved.
4. Revert the ess/system citation paths that followed the rename of `connection.rs` once the file is restored.

The alternative — updating the sha pins to bless the mutated readers — changes published wire bytes without a version migration and is refused by the approved design.
