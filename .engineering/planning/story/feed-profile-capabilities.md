---
format: aep.planning-md/3
id: story:feed-profile-capabilities
kind: story
status: draft
title: A feed profile declares what its provider observes, and the suite tests that
relations:
- decomposes: epic:generic-datasource-feeds
- serves: vision:independent-contract-adapters
- depends_on: story:feed-contract
scope:
- confidence: inferred
  path: adapters/catalog/spec/ess/domains/feed.yaml
- confidence: inferred
  path: adapters/catalog/src/feed.rs
- confidence: inferred
  path: adapters/catalog/tests/feed
- confidence: inferred
  path: contracts/catalog/v1alpha1/semantics.md
- confidence: inferred
  path: contracts/datasources/feed/v1alpha1/scenarios
- confidence: cited
  path: contracts/datasources/feed/v1alpha1/semantics.md
- confidence: cited
  path: crates/connectors-build/src/catalog_feed_conformance.rs
- confidence: cited
  path: crates/connectors-build/src/feed_conformance.rs
- confidence: cited
  path: ess/domains/feed.yaml
revision: 8
---
## Outcome

A feed profile declares what its provider can observe, and the family's conformance suite tests a binding against what its profile declares rather than against the strongest provider: whether deletions are observed, where a container's `kind` comes from, and how a revision is formed.

## Why

Wave 20261006d (2026-10-06): the GitLab binding passed 21 of 28 scenarios with no defect in it. The suite always expects a tombstone, while `contracts/datasources/feed/v1alpha1/semantics.md:134-135` lets a profile observe no deletions (GitLab deletes merge requests outright); it expects the stored `kind`, while GitLab's word is always `project`; and its restore scenario reuses an `updated_at`, which is GitLab's revision.

## Work

- Contract and model (`ess/domains/feed.yaml`): a profile's capabilities — `deletions: observed | not-observed`, `kind: provider-word | fixed-word`, `revision: opaque | update-time`, `visibility: mapped | all-private` — carried in each binding's descriptor.
- The suite synthesizes or skips the deletion, kind, restore and public-listing scenarios from those capabilities, and a binding cannot claim a capability its declaration does not support.
- Added 2026-10-07: `visibility`. `story:gitlab-feed-binding` lists every project `private` (decided 2026-10-06), and the authored scenario `contracts/datasources/feed/v1alpha1/scenarios/listing-omits-direct-conversation.yaml:47-49` expects a container listed `public`; a profile declaring `all-private` is not held to it.
- A skip cannot travel through `ess-conformance`: a Rust producer refuses `skipped != 0` (`crates/verify/ess-conformance/src/counts.rs:460` at ESS 0.55.0). The suite for a profile therefore leaves the inapplicable scenarios out before admission, and the harness names each left-out scenario, with the capability that left it out, in its own report.

## Acceptance

- A fixture binding declaring `deletions: not-observed` passes the suite with the tombstone scenarios left out and named in the harness report; declaring `observed` and dropping tombstones fails it.
- A fixture binding declaring `visibility: all-private` passes with the public-listing scenario left out and named; declaring `mapped` and listing everything `private` fails it.
- The native fixture and the two catalog fixture providers still pass every scenario they declare, and the count of scenarios each runs is printed beside the 28 the family defines.

## Scope

Derived 2026-10-07 by `aep:story-scoper`. **Cited** = read from the story or the tree; **inferred** = a reading that could be wrong.

- **Primary surface:** the feed suite harness in `crates/connectors-build` — cited (`contracts/datasources/feed/v1alpha1/semantics.md:214`)
- **Files:** `crates/connectors-build/src/feed_conformance.rs`: `EXPECTED_SCENARIOS = 28` (36-38), `suite()` (49-86), `trait Binding` (142-158; `identity()` returns only implementation and profile), `Native` (161-187, profile `fixture-feed/1`), `run` (345-357), `rows` compares `kind` (632) and `Deleted` (640), the test (805-822) — cited
- **Files:** `crates/connectors-build/src/catalog_feed_conformance.rs`: `declared()` (52-74), `Simulated` (115-274), `Catalog::identity` (336-338), the 28-of-28 test (478-507) — cited
- **Files:** `ess/domains/feed.yaml` (`connectors.feed`): `OperationBinding {id, contract, profile}` (45-55), `FeedItem` lifecycle (192-205), `RemoveItem`/`RestoreItem` (331-391); `feed-binding` component (`ess/components.yaml:75`) — cited
- **Files:** `contracts/datasources/feed/v1alpha1/semantics.md`: profile statement (17-26), revisions (114-122), deletions (124-135), model and conformance (197-216) — cited
- **Also likely:** `adapters/catalog/src/feed.rs` (`Items.deleted` 183-184, `Declaration` 190-195, `Feed::new` refusals 311-400, `declarations()` 905-1028); `adapters/catalog/spec/ess/domains/feed.yaml` (74-75, 138-139, 141-152); `adapters/catalog/tests/feed/{time,cursor}.operations.json`; `contracts/catalog/v1alpha1/semantics.md` §3.3 (127-170); `contracts/datasources/feed/v1alpha1/scenarios/` — inferred
- **Confidence:** medium. Open: whether the capabilities ride the wire `connectors_core::Operation` (`crates/connectors-core/src/lib.rs:83-92`, `deny_unknown_fields`, 18 importers) or stay in the feed model and the catalog declaration; the implementor decides in the ESS model first and reports which.
- **Would collide with:** `story:gitlab-feed-binding` and `story:catalog-feed-engine-extensions` on `adapters/catalog/src/feed.rs`, `adapters/catalog/spec/ess/domains/feed.yaml`, §3.3 and `crates/connectors-build/src/catalog_feed_conformance.rs`; it lands first.
- **Safety fact:** the catalog ESS comment calls `kind` the provider's word (`adapters/catalog/spec/ess/domains/feed.yaml:74-75`) while the engine reads it as a pointer (`feed.rs:329,583`) and so does §3.3 (`contracts/catalog/v1alpha1/semantics.md:140`) — cited; `story:gitlab-feed-binding` resolves it.
