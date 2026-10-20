---
format: aep.planning-md/3
id: review-result:definitions-only-plan-parallel-r2
kind: review-result
status: active
title: Plan critic parallel safety, round 2, on the definitions-only drafts
relations:
- reviews: story:definitions-release-index
- reviews: story:definitions-install
- reviews: story:definitions-pin-covers-catalog
- reviews: story:definition-tests-read-pinned-release
- reviews: story:catalog-tests-read-pinned-release-gitlab-engine
- reviews: story:catalog-tests-read-pinned-release-providers
- reviews: story:definitions-fixture-copy-removed
- reviews: story:instances-resolve-through-locked-index
revision: 1
---
needs-revision
story:instances-resolve-through-locked-index — it rewrites the compiled-in instance path (`adapters/catalog/src/local.rs`, `adapters/catalog/build.rs`), which the providers batch's `local_runtime.rs` and google read tests and the gitlab-engine batch's `runpod.rs` and gitlab tests keep using. Its Scope names none of those files, and no `depends_on` edge orders it against `story:catalog-tests-read-pinned-release-providers` or `story:catalog-tests-read-pinned-release-gitlab-engine`. In the providers batch, `local_runtime.rs` is also the parent of `derived_instance.rs`, the file this story edits (`use super::*`, one test binary). Both batches' "rename the directories after build" acceptance also reads the `build.rs` behaviour this story changes (cited, mixed). Fix either by an ordering edge that records the shared test binary and `build.rs` as its reason, or by splitting the surface so the story names every test file it must change; I do not choose between them. — .engineering/planning/story/instances-resolve-through-locked-index.md:56-66, .engineering/planning/story/catalog-tests-read-pinned-release-providers.md:30-31, .engineering/planning/story/catalog-tests-read-pinned-release-gitlab-engine.md:45-46
story:instances-resolve-through-locked-index — its Scope names no manifest or `Cargo.lock`, yet the catalog provider and the host must read the lock and cache that `story:definitions-install` creates (probably through its library crate, which would need a new dependency). `story:definitions-pin-covers-catalog` owns `adapters/catalog/Cargo.toml`, `apps/connectors/Cargo.toml` and `Cargo.lock`, and the two stories share only the install dependency, so one wave can hold both (inferred). Fix either by an ordering edge that records the manifests as its reason, or by stating in the body that the story adds no dependency. — .engineering/planning/story/instances-resolve-through-locked-index.md:56-66, .engineering/planning/story/definitions-pin-covers-catalog.md:39-41

Round-1 finding: fixed. `story:definitions-pin-covers-catalog` now adds the dev-dependency to every consuming manifest and `Cargo.lock` (definitions-pin-covers-catalog.md, Outcome third bullet). Both batch bodies say they touch no manifest or `Cargo.lock`. The two batches are still file-disjoint: 45 and 35 files, empty `comm` intersection, no `#[path]` or `include!` link between them.

What I read: 8 drafts and the epic with `aep plan artifact show` and `graph`, plus review-result:definitions-only-plan-parallel-r1. I also ran `git grep` at 9201b1177 over `adapters/catalog/tests` (`providers|generated/bundles`, `#[path]`, derived-instance symbols).
- **Surfaces.** 5 of 8 are mostly cited: the two test batches, `definition-tests`, `fixture-copy` and `instances` (partly inferred). 3 are partly inferred: `release-index`, `install` and `pin-covers-catalog`. None are unplaceable.
- **Producers.** I traced 13 acceptance reads to a producer in the set. 9 are direct `depends_on` edges: install→release-index, pin→install, instances→install, the three test stories→pin and fixture-copy→those three. 4 are transitive only, all through pin→install→release-index: pin reading the first tag, `definition-tests` reading Jira at that tag, and the two batches reading the real release.

What I could not establish:
- **`aep plan artifact waves`.** I did not rely on it, because no frontmatter scope exists.
- **Install and test-support crates.** Their locations are left to the implementor, so the manifest finding rests on an inferred surface.
- **Out of my lane, scope:** `engine.rs` matches the `providers` search only through the word in a function name. It is not a gap.
- **Out of my lane, design:** `definitions-install` would own lock and cache paths in `crates/connectors-host/src/local/`, and `instances` also lists `crates/connectors-host/src/local/config.rs`. An edge orders the two stories.

```findings
[{"file": ".engineering/planning/story/instances-resolve-through-locked-index.md", "line": 56, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "this story rewrites the compiled-in instance path (adapters/catalog/src/local.rs, build.rs) that the providers batch (local_runtime.rs, google read tests) and the gitlab-engine batch (runpod.rs, gitlab tests) keep using; its Scope names none of those test files, derived_instance.rs shares the local_runtime test binary with providers-batch local_runtime.rs (use super::*), both batches' rename-after-build acceptance reads the build.rs behaviour it changes, and no depends_on edge orders it against either batch; remedies: an ordering edge recording the shared test binary and build.rs as its reason, or splitting the surface so the story names every test file it changes"},
 {"file": ".engineering/planning/story/instances-resolve-through-locked-index.md", "line": 56, "category": "parallel-safety", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "Scope names no manifest or Cargo.lock although the provider and host must read the lock and cache that story:definitions-install creates (likely a new dependency on its library crate), while story:definitions-pin-covers-catalog owns adapters/catalog/Cargo.toml, apps/connectors/Cargo.toml and Cargo.lock and the two share only the install dependency; this rests on an inferred surface (the install crate's location and the dependency are not stated); remedies: an ordering edge recording the manifests as its reason, or a statement in the body that the story adds no dependency"}]
```
