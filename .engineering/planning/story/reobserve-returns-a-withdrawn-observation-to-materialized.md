---
format: aep.planning-md/1
id: story:reobserve-returns-a-withdrawn-observation-to-materialized
kind: story
status: draft
title: A withdrawn observation that was materialized comes back as materialized
relations:
- informed_by: review-result:adversary-ess-domain-pass-1
scope:
- confidence: cited
  path: crates/catalog-build/tests/main/ess_citation_fence.rs
- confidence: cited
  path: crates/integration-monitoring/src/backend.rs
revision: 3
---
# Story: a withdrawn observation that was materialized comes back as materialized

## What was measured

`crates/integration-monitoring/src/backend.rs:1558-1565` re-activates an observation the refresh
pass has seen again. It sets `active = true` and updates the evidence, and it does **not** clear
`connection_ref`.

`observation_summary` (`:1656-1665`) derives the projected state in this order: `!active` →
`Withdrawn`, then `connection_ref.is_some()` → `Materialized`, then `target_provider.is_some()` →
`Observed`. `materialize` is the only thing that sets `connection_ref` (`:1244`).

So an observation that was materialized, then withdrawn — `degrade_hosted_targets` at `:1359-1364`
sets `active = false` on every observation — and then seen again by the refresh pass comes back as
**`Materialized`**, not `Observed`.

Measured by an adversary pass against the ESS specification of this domain, which had declared the
transition as `reobserve: Withdrawn -> Observed`. The specification now carries the observed
behaviour as `rematerialize: Withdrawn -> Materialized`
(`ess/system/domains/connection.yaml`), because a specification that states a transition the tree
does not perform is worse than one that states an awkward truth.

## The open question

Is `Withdrawn -> Materialized` the behaviour anyone wants? A withdrawn observation's Connection was
built against evidence that has since changed. Coming back as `Materialized` asserts the old
Connection still describes it, and nothing re-checks that.

## What reaches it

**Not established.** No test covers *refresh a materialized observation after it was withdrawn*.
`cargo test -p integration-monitoring --locked` is 16/16 green both with and without the one-line
fix, which is exactly the problem: the suite does not distinguish the two behaviours.

## Why it was not simply fixed

A one-line fix exists and was measured — `observation.connection_ref = None;` in that branch — but
it has an uncovered surface. Clearing the reference leaves the child in `state.children`
(`:1191-1195` returns it) and `child_is_current` (`:1684-1690`) never reads `connection_ref`, so a
later `materialize` may mint a duplicate child. That is a behaviour decision on a shipping adapter,
and it was refused inside a wave whose unit writes YAML.

## Shape

Decide which of the two the adapter should do, then make the suite tell them apart:

- **Clear it** — a re-seen observation is re-derived from evidence, so a stale Connection reference
  should not survive. Requires deciding what happens to the orphaned child in `state.children`.
- **Keep it** — a Connection built once stays until something withdraws it explicitly. Then
  `rematerialize` is correct and permanent, and the story closes as a no-op with a test.

## Acceptance

- A test drives materialize → withdraw → refresh, and asserts the resulting projected state.
- Whichever branch is chosen, `crates/catalog-build/tests/main/ess_citation_fence.rs`'s
  `the_reobserve_site_leaves_a_connection_ref_and_the_specification_says_so` is updated in the same
  change, and `ess/system/domains/connection.yaml` agrees with the adapter.

## Provenance

`review-result:adversary-ess-domain-pass-1`, finding 6. The finding was `origin: introduced` against
the specification and is fixed there; this story carries the behaviour question it exposed, which is
older than the specification.
