---
format: aep.planning-md/3
id: review-result:wave-20261106-u2-security
kind: review-result
status: active
title: Wave 2026-11-06 U2 security review
relations:
- reviews: story:owner-bounded-read-realization-matches-projection
revision: 1
---
unit: U2 security (conn-w1106-u2; unit diff da0371c72..f42d53975; findings cover HEAD 5f6f513b2)
verdict: green
cases: executed 13→16, red 0
origin: introduced 1, pre-existing 0, undecided 0
wrote-outside-worktree: ~/.cache/connectors-wave-20261106/u2/sec/ (cases-alone{,-2,-3,-4}.log, suite.log, fmt.log, fmt-2.log, clippy.log, suite.sh, commit-msg.pXyiaY)
needs-coordinator: no

**Diff, my commit only** (`git diff --stat 9a73f986e..5f6f513b2`; both are test paths):
- `crates/connectors-host/src/local/owner/governed/tests.rs`: +1 line, the `mod` registration.
- `crates/connectors-host/src/local/owner/governed/tests/bounded_realization_security.rs`: new, 328 lines.

Commit `5f6f513b2` "owner: security cases for the bounded read's realization rule". Author and committer are both b10x-bot[bot]. Only my two paths are in it; the adversary's `9a73f986e` is theirs.

**Cases added.** All three are green against the unit's code. Run alone: `test result: ok. 3 passed; 0 failed; ... 718 filtered out; finished in 46.79s`.
- **`bounded_owner_serves_exactly_the_realizations_the_projection_serves`**: 23 combinations driven through `bounded::resolve` and `bounded::read`.
  - From the served implemented `resource` read, it changes one condition at a time: the profile, or a non-inline input or output schema (`$ref`, `$id`, `$anchor`, `$dynamicRef`, `$recursiveRef`). One combination has both conditions false.
  - It also covers the generic realization on each profile and no realization at all.
  - Each refusal gives `unsupported`, an early-refusal audit that names no operation, and 0 provider requests. Exactly 3 combinations dispatch, once each.
- **`worker_admission_refuses_an_implemented_realization_the_projection_would_not_serve`**: the serialized worker's admission (`supervisor.rs:1064`) refuses the same 14 implemented combinations as `unsupported` before any provider starts. The served one gets through admission and then fails at provider startup.
- **`relocated_schemas_are_admitted_by_the_descriptor_cache`**: a precondition check. My first two runs failed on my own fixture, not on the unit's code: the descriptor cache refuses a draft 2020-12 schema that uses `$recursiveRef` (`MetadataUnavailable`), so that form can never reach the owner. The case now declares draft 2019-09.

**Suite run, after the cases existed:** `cargo.sh <tree> test --locked -p connectors-host --lib -- owner::governed::tests::bounded` gave `test result: ok. 16 passed; 0 failed; ... 705 filtered out; finished in 51.74s`, EXIT=0. The before count of 13 is the implementor's `green-bounded.log` with the same filter.

**Checks:**
- `clippy --locked -p connectors-host --all-targets -- -D warnings`: Finished, EXIT=0.
- `fmt --check -p connectors-host`: EXIT=0 after I applied rustfmt to my own file and reordered my `mod` line. Neither the suite nor clippy was rerun after that, because the change was formatting and declaration order only.
- `target/` is 2.5G. Wall time across 8 wrapper calls was about 28 min including build-slot waits; the wrapper does not separate cargo time from waiting. Slot waits were about 3 min that I know of, so cargo time may have passed the 20-minute "tell" limit. It stayed under the 30-minute stop.

**Finding U2-SEC-1 (note, contract-drift, CONFIRMED, introduced).**
- What was measured: `crates/connectors-core/src/lib.rs:103-104`. The new public doc on `inline_schema` says "the owner serves one, only over inline schemas". But the owner's generic arm at `bounded.rs:61` serves any schema. I checked this by reading; I added no red case, because that behaviour is the item already filed out of scope.
- What reaches it: any reader of the public `connectors_core::inline_schema`.
- Correction, not applied: limit the sentence to the implemented realization until the filed generic item lands.

**Safety fact.** Every budgeted read reaches dispatch only through `bounded::resolve` with a budget:
- `governed_action` calls `bounded::read`, which calls `read_using`, which resolves at `governed.rs:201` and again at `:275`.
- The worker admission at `supervisor.rs:1064` and the final fence at `:1699` call the same function.
- Proven by running tests for the coordinator and the worker admission. The final fence is checked only by pointing at the line.

**Reviewed and could not fault:**
- `inline_schema` was moved without a single change, so the owner and the projection now share one definition.
- The owner and the projection read the same descriptor: describe only drops operations (`metadata.rs:48-63`).
- Plan changes between admission and dispatch are caught: the fingerprint covers config and bootstrap (`governed.rs:153`), and resolve runs again before dispatch.
- The projection's Read-requirement condition holds on the owner side too (`governed.rs:140`).
- Schemas that do not compile, including remote `$ref`, are never cached (`runtime.rs:826-833`), so they cannot reach the owner.
- The unbudgeted projected `GovernedRead` frame has no realization rule, but it permits nothing a plain governed read does not already permit.

**Not settled:** the implementor's red log stops at the first iteration of each loop (`generic-http`, input `$ref`). I had no base worktree, so I did not run my cases against the base.

**CHANGELOG line I would add:** none; this commit adds tests only.

```findings
[{"file":"crates/connectors-core/src/lib.rs","line":103,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"U2-SEC-1: the new inline_schema doc says the owner serves a bounded read only over inline schemas, but the owner's generic arm (bounded.rs:61) serves any schema."}]
```

(Coordinator: home-directory paths shortened to ~ for the repository's personal-path gate; nothing else changed.)
