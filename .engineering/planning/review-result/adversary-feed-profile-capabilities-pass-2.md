---
format: aep.planning-md/3
id: review-result:adversary-feed-profile-capabilities-pass-2
kind: review-result
status: active
title: Adversary pass 2 on feed profile capabilities
relations:
- reviews: story:feed-profile-capabilities
revision: 1
---
unit: U1 story:feed-profile-capabilities (2f33e69ba plus my uncommitted test additions)
verdict: red
cases: executed 21→25, red 4
origin: introduced 4, pre-existing 0, undecided 0
wrote-outside-worktree: none
needs-coordinator: no

```
 crates/connectors-build/src/feed_conformance.rs | 179 +++  (a #[cfg(test)] module, main.rs:21-22; tests appended only)
```

The correction holds update-time bindings to nothing about revisions. The revision field is masked, and the `excludes … revision: r1` steps are dropped whole. That removes the only check that a changed item is read again.

| # | file:line | What breaks | Origin | Failing test |
|---|---|---|---|---|
| F5 | crates/connectors-build/src/feed_conformance.rs:232 | Test binding: `deletions: not-observed`, `revision: update-time`, watermark by creation time, so an item changed after it was read is never read again. It passes **23 of 23**. In `resumed-read` only `item-1 … state: Present` is left, and the stale item satisfies it. The same binding claiming `opaque` fails `resumed-read` (25 run, 23 passed). This contradicts `semantics.md:229` ("loses a change on resume then fails"). It reaches every update-time profile, and the engine requires that claim whenever the revision pointer is `updated_at` (the GitLab case). I built the lossy binding through the native `Binding` trait. The simulated catalog provider has no creation-time filter to build it with. | introduced | `feed_conformance::adversary2_an_update_time_binding_that_loses_a_change_on_resume_fails` |
| F6 | crates/connectors-build/src/feed_conformance.rs:175 | Native binding: claims `update-time`, but its revision is `created_at` and never changes. It passes **26 of 26**, because nothing compares the claim with anything. The catalog route catches the same mistake: a declaration reading `/created` for both pointers fails 4 synthesized revise and remove scenarios, which compare `updated_at`. So only a native binding reaches it. | introduced | `feed_conformance::adversary2_an_update_time_claim_is_held_to_the_update_time` |
| F7 | crates/connectors-build/src/feed_conformance.rs:174 | `kind: fixed-word`: a binding listing each container's id as its kind passes **28 of 28**. `semantics.md:77` says "the one word the profile states", but `ProfileCapabilities` carries no word, so the suite cannot check it. Today only a native binding can reach it, since the engine refuses `fixed-word`. | introduced | `feed_conformance::adversary2_a_fixed_word_claim_is_held_to_one_word` |
| F8 | crates/connectors-build/src/feed_conformance.rs:159 | `deletions: not-observed`: a binding that still reports tombstones passes **25 of 25**. `semantics.md:150` says such a binding "never reports `deleted: true`". The catalog cannot under-claim this (`supports()` ties the claim to `items.deleted`), so only a native binding reaches it. | introduced | `feed_conformance::adversary2_a_not_observed_claim_is_held_to_reporting_no_tombstone` |

Suggested fixes (I did not apply them):
- **F5 and F6:** under `update-time`, hold `revision` to the instant of the item's last effective change instead of masking it. The harness already resolves that instant at `:279`. Rewrite the `excludes … r1` into its instant rather than dropping it.
- **F7:** carry the fixed word in the profile, or check that every listed container has one kind.
- **F8:** under `not-observed`, turn the tombstone `contains state: Deleted` into an `excludes`, instead of leaving the scenario out.

**Each case run alone, before any suite run (exit 101 each):**
- F5: `ran 23 of the 28 scenarios the family defines (23 passed)` / `a binding that never reads a changed item again passed under revision: update-time (failed: {})`
- F6: `ran 26 of the 28 … (26 passed)` / `a revision that never changes passed under revision: update-time`
- F7: `ran 28 of the 28 … (28 passed)` / `a kind that differs per container passed under kind: fixed-word`
- F8: `ran 25 of the 28 … (25 passed)` / `a binding reporting tombstones passed under deletions: not-observed`

**Suites, run after the cases existed:**
- `cargo test --locked -p connectors-build feed`: 21 passed, 4 failed (exactly the 4 above), exit 101. The 21 before is the implementor's count in 2f33e69ba.
- `cargo test --locked -p connectors-catalog-provider --no-fail-fast`: 376 passed, 0 failed, exit 0. The known flake did not fire.

**Attacked and not broken:**
- **All-private:** a direct conversation is still caught; both `excludes` steps are kept. A stored-private container listed `public` still fails.
- **Masking scope:** no `excludes` names `kind` or `visibility`. Under the weakest profile no kept scenario loses all its view expectations, and 23 scenarios run.
- **Report:** run plus left out came to 28 in every run, and every masked or left-out entry matches a changed step.
- **Catalog, honest weaker declarations:** I probed update-time, all three weak values together, and not-observed with all-private, on both the cursor and time forms. All pass. I removed the probe because it was green.
- **Catalog, wrong pointers:** a declaration reading `/created` for revision and `updated_at` fails 4 scenarios. Probe removed.
- **Engine load checks:** `supports()` is unchanged since pass 1, and nothing refuses `updated_at` pointing at the creation field.

**Paths outside the worktree:** none. I deleted my scratch directory `.local/tmp/u1-adv2`. `.local/tmp/u1` is not mine and I left it. As in pass 1, I took no worktree lease.

```findings
[
  {"file": "crates/connectors-build/src/feed_conformance.rs", "line": 232, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "F5: under revision: update-time the masked revision and the dropped excludes leave no check that a changed item is read again, so a binding that loses a change on resume passes 23 of 23 while the same binding claiming opaque fails resumed-read, contrary to semantics.md:229"},
  {"file": "crates/connectors-build/src/feed_conformance.rs", "line": 175, "category": "mutant", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "F6: an update-time claim is never compared with the update time, so a native binding whose revision is created_at passes 26 of 26; the catalog route catches it through updated_at in 4 synthesized scenarios"},
  {"file": "crates/connectors-build/src/feed_conformance.rs", "line": 174, "category": "contract-drift", "severity": "warning", "verdict": "INFEASIBLE", "origin": "introduced", "message": "F7: kind: fixed-word masks kind with no word to hold it to, so a binding listing a different kind per container passes 28 of 28 against semantics.md:77; only native bindings reach fixed-word today"},
  {"file": "crates/connectors-build/src/feed_conformance.rs", "line": 159, "category": "contract-drift", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "F8: deletions: not-observed leaves out the tombstone scenarios instead of holding the binding to reporting none, so a native binding that reports tombstones passes 25 of 25 against semantics.md:150"}
]
```
